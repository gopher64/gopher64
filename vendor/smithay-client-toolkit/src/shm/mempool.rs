use std::{
    cell::RefCell,
    ffi::CStr,
    fmt,
    fs::File,
    io,
    os::unix::io::{FromRawFd, RawFd},
    rc::Rc,
    time::SystemTime,
    time::UNIX_EPOCH,
};

#[cfg(target_os = "linux")]
use nix::sys::memfd;
use nix::{
    errno::Errno,
    fcntl,
    sys::{mman, stat},
    unistd,
};

use memmap2::MmapMut;

use wayland_client::{
    protocol::{wl_buffer, wl_shm, wl_shm_pool},
    Attached, Main,
};

/// A Double memory pool, for convenient double-buffering
///
/// This type wraps two internal memory pool, and can be
/// use for conveniently implementing double-buffering in your
/// apps.
///
/// DoubleMemPool requires a implementation that is called when
/// one of the two internal memory pools becomes free after None
/// was returned from the `pool()` method.
#[derive(Debug)]
pub struct DoubleMemPool {
    pool1: MemPool,
    pool2: MemPool,
    free: Rc<RefCell<bool>>,
}

impl DoubleMemPool {
    /// Create a double memory pool
    pub fn new<F>(shm: Attached<wl_shm::WlShm>, callback: F) -> io::Result<DoubleMemPool>
    where
        F: FnMut(wayland_client::DispatchData) + 'static,
    {
        let free = Rc::new(RefCell::new(true));
        let callback = Rc::new(RefCell::new(callback));
        let my_free = free.clone();
        let my_callback = callback.clone();
        let pool1 = MemPool::new(shm.clone(), move |ddata| {
            let signal = {
                let mut my_free = my_free.borrow_mut();
                if !*my_free {
                    *my_free = true;
                    true
                } else {
                    false
                }
            };
            if signal {
                (my_callback.borrow_mut())(ddata);
            }
        })?;
        let my_free = free.clone();
        let pool2 = MemPool::new(shm, move |ddata| {
            let signal = {
                let mut my_free = my_free.borrow_mut();
                if !*my_free {
                    *my_free = true;
                    true
                } else {
                    false
                }
            };
            if signal {
                (callback.borrow_mut())(ddata);
            }
        })?;
        Ok(DoubleMemPool { pool1, pool2, free })
    }

    /// This method checks both its internal memory pools and returns
    /// one if that pool does not contain any buffers that are still in use
    /// by the server. If both the memory pools contain buffers that are currently
    /// in use by the server None will be returned.
    pub fn pool(&mut self) -> Option<&mut MemPool> {
        if !self.pool1.is_used() {
            Some(&mut self.pool1)
        } else if !self.pool2.is_used() {
            Some(&mut self.pool2)
        } else {
            *self.free.borrow_mut() = false;
            None
        }
    }
}

#[derive(Debug)]
struct Inner {
    file: File,
    len: usize,
    pool: Main<wl_shm_pool::WlShmPool>,
    mmap: MmapMut,
}

impl Inner {
    fn new(shm: Attached<wl_shm::WlShm>) -> io::Result<Self> {
        let mem_fd = create_shm_fd()?;
        let mem_file = unsafe { File::from_raw_fd(mem_fd) };
        mem_file.set_len(4096)?;

        let pool = shm.create_pool(mem_fd, 4096);

        let mmap = unsafe { MmapMut::map_mut(&mem_file).unwrap() };

        Ok(Inner { file: mem_file, len: 4096, pool, mmap })
    }

    fn resize(&mut self, newsize: usize) -> io::Result<()> {
        if newsize > self.len {
            self.file.set_len(newsize as u64)?;
            self.pool.resize(newsize as i32);
            self.len = newsize;
            self.mmap = unsafe { MmapMut::map_mut(&self.file).unwrap() };
        }
        Ok(())
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.pool.destroy();
    }
}

/// A wrapper handling an SHM memory pool backed by a shared memory file
///
/// This wrapper handles for you the creation of the shared memory file and its synchronization
/// with the protocol.
///
/// Mempool internally tracks the release of the buffers by the compositor. As such, creating a buffer
/// that is not commited to a surface (and then never released by the server) would cause the Mempool
/// to be stuck believing it is still in use.
///
/// Mempool will also handle the destruction of buffers and as such the `destroy()` method should not
/// be used on buffers created from Mempool.
///
/// Overwriting the contents of the memory pool before it is completely freed may cause graphical
/// glitches due to the possible corruption of data while the compositor is reading it.
///
/// Mempool requires a callback that will be called when the pool becomes free, this
/// happens when all the pools buffers are released by the server.
pub struct MemPool {
    inner: Inner,
    buffer_count: Rc<RefCell<u32>>,
    callback: Rc<RefCell<dyn FnMut(wayland_client::DispatchData)>>,
}

impl MemPool {
    /// Create a new memory pool associated with given shm
    pub fn new<F>(shm: Attached<wl_shm::WlShm>, callback: F) -> io::Result<MemPool>
    where
        F: FnMut(wayland_client::DispatchData) + 'static,
    {
        Ok(MemPool {
            inner: Inner::new(shm)?,
            buffer_count: Rc::new(RefCell::new(0)),
            callback: Rc::new(RefCell::new(callback)) as Rc<RefCell<_>>,
        })
    }

    /// Resize the memory pool
    ///
    /// This affect the size as it is seen by the wayland server. Even
    /// if you extend the temporary file size by writing to it, you need to
    /// call this method otherwise the server won't see the new size.
    ///
    /// Memory pools can only be extented, as such this method will do nothing
    /// if the requested new size is smaller than the current size.
    ///
    /// This method allows you to ensure the underlying pool is large enough to
    /// hold what you want to write to it.
    pub fn resize(&mut self, newsize: usize) -> io::Result<()> {
        self.inner.resize(newsize)
    }

    /// Create a new buffer to this pool
    ///
    /// The parameters are:
    ///
    /// - `offset`: the offset (in bytes) from the beginning of the pool at which this
    ///   buffer starts
    /// - `width`: the width of this buffer (in pixels)
    /// - `height`: the height of this buffer (in pixels)
    /// - `stride`: distance (in bytes) between the beginning of a row and the next one
    /// - `format`: the encoding format of the pixels. Using a format that was not
    ///   advertised to the `wl_shm` global by the server is a protocol error and will
    ///   terminate your connection
    pub fn buffer(
        &self,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
    ) -> wl_buffer::WlBuffer {
        *self.buffer_count.borrow_mut() += 1;
        let my_buffer_count = self.buffer_count.clone();
        let my_callback = self.callback.clone();
        let buffer = self.inner.pool.create_buffer(offset, width, height, stride, format);
        buffer.quick_assign(move |buffer, event, dispatch_data| match event {
            wl_buffer::Event::Release => {
                buffer.destroy();
                let new_count = {
                    // borrow the buffer_count for as short as possible, in case
                    // the user wants to create a new buffer from the callback
                    let mut my_buffer_count = my_buffer_count.borrow_mut();
                    *my_buffer_count -= 1;
                    *my_buffer_count
                };
                if new_count == 0 {
                    (my_callback.borrow_mut())(dispatch_data);
                }
            }
            _ => unreachable!(),
        });
        (*buffer).clone().detach()
    }

    /// Uses the memmap2 crate to map the underlying shared memory file
    pub fn mmap(&mut self) -> &mut MmapMut {
        &mut self.inner.mmap
    }

    /// Returns true if the pool contains buffers that are currently in use by the server
    pub fn is_used(&self) -> bool {
        *self.buffer_count.borrow() != 0
    }
}

impl fmt::Debug for MemPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemPool")
            .field("inner", &self.inner)
            .field("buffer_count", &self.buffer_count)
            .field("callback", &"Fn() -> { ... }")
            .finish()
    }
}

impl io::Write for MemPool {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::Write::write(&mut self.inner.file, buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.inner.file)
    }
}

impl io::Seek for MemPool {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        io::Seek::seek(&mut self.inner.file, pos)
    }
}

/// A wrapper handling an SHM memory pool backed by a shared memory file
///
/// This wrapper handles the creation of the shared memory file, its synchronization with the
/// protocol, and the allocation of buffers within the pool.
///
/// AutoMemPool internally tracks the release of the buffers by the compositor. As such, creating a
/// buffer that is not committed to a surface (and then never released by the server) would result
/// in that memory being unavailable for the rest of the pool's lifetime.
///
/// AutoMemPool will also handle the destruction of buffers; do not call destroy() on the returned
/// WlBuffer objects.
///
/// The default alignment of returned buffers is 16 bytes; this can be changed by using the
/// explicit with_min_align constructor.
#[derive(Debug)]
pub struct AutoMemPool {
    inner: Inner,
    align: usize,
    free_list: Rc<RefCell<Vec<(usize, usize)>>>,
}

impl AutoMemPool {
    /// Create a new memory pool associated with the given shm
    pub fn new(shm: Attached<wl_shm::WlShm>) -> io::Result<AutoMemPool> {
        Self::with_min_align(shm, 16)
    }

    /// Create a new memory pool associated with the given shm.
    ///
    /// All buffers will be aligned to at least the value of (align), which must be a power of two
    /// not greater than 4096.
    pub fn with_min_align(shm: Attached<wl_shm::WlShm>, align: usize) -> io::Result<AutoMemPool> {
        assert!(align.is_power_of_two());
        assert!(align <= 4096);
        let inner = Inner::new(shm)?;
        let free_list = Rc::new(RefCell::new(vec![(0, inner.len)]));
        Ok(AutoMemPool { inner, align, free_list })
    }

    /// Resize the memory pool
    ///
    /// This is normally done automatically, but can be used to avoid multiple resizes.
    pub fn resize(&mut self, new_size: usize) -> io::Result<()> {
        let old_size = self.inner.len;
        if old_size >= new_size {
            return Ok(());
        }
        self.inner.resize(new_size)?;
        // add the new memory to the freelist
        let mut free = self.free_list.borrow_mut();
        if let Some((off, len)) = free.last_mut() {
            if *off + *len == old_size {
                *len += new_size - old_size;
                return Ok(());
            }
        }
        free.push((old_size, new_size - old_size));
        Ok(())
    }

    fn alloc(&mut self, size: usize) -> io::Result<usize> {
        let mut free = self.free_list.borrow_mut();
        for (offset, len) in free.iter_mut() {
            if *len >= size {
                let rv = *offset;
                *len -= size;
                *offset += size;
                return Ok(rv);
            }
        }
        let mut rv = self.inner.len;
        let mut pop_tail = false;
        if let Some((start, len)) = free.last() {
            if start + len == self.inner.len {
                rv -= len;
                pop_tail = true;
            }
        }
        // resize like Vec::reserve, always at least doubling
        let target = std::cmp::max(rv + size, self.inner.len * 2);
        self.inner.resize(target)?;
        // adjust the end of the freelist here
        if pop_tail {
            free.pop();
        }
        if target > rv + size {
            free.push((rv + size, target - rv - size));
        }
        Ok(rv)
    }

    fn free(free_list: &RefCell<Vec<(usize, usize)>>, mut offset: usize, mut len: usize) {
        let mut free = free_list.borrow_mut();
        let mut nf = Vec::with_capacity(free.len() + 1);
        for &(ioff, ilen) in free.iter() {
            if ioff + ilen == offset {
                offset = ioff;
                len += ilen;
                continue;
            }
            if ioff == offset + len {
                len += ilen;
                continue;
            }
            if ioff > offset + len && len != 0 {
                nf.push((offset, len));
                len = 0;
            }
            if ilen != 0 {
                nf.push((ioff, ilen));
            }
        }
        if len != 0 {
            nf.push((offset, len));
        }
        *free = nf;
    }

    /// Create a new buffer in this pool
    ///
    /// The parameters are:
    ///
    /// - `width`: the width of this buffer (in pixels)
    /// - `height`: the height of this buffer (in pixels)
    /// - `stride`: distance (in bytes) between the beginning of a row and the next one
    /// - `format`: the encoding format of the pixels. Using a format that was not
    ///   advertised to the `wl_shm` global by the server is a protocol error and will
    ///   terminate your connection
    pub fn buffer(
        &mut self,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
    ) -> io::Result<(&mut [u8], wl_buffer::WlBuffer)> {
        let len = (height as usize) * (stride as usize);
        let alloc_len = (len + self.align - 1) & !(self.align - 1);
        let offset = self.alloc(alloc_len)?;
        let offset_i = offset as i32;
        let buffer = self.inner.pool.create_buffer(offset_i, width, height, stride, format);
        let free_list = self.free_list.clone();
        buffer.quick_assign(move |buffer, event, _| match event {
            wl_buffer::Event::Release => {
                buffer.destroy();
                Self::free(&free_list, offset, alloc_len);
            }
            _ => unreachable!(),
        });
        Ok((&mut self.inner.mmap[offset..][..len], buffer.detach()))
    }

    /// Try drawing with the given closure
    ///
    /// This is identical to buffer(), but will only actually create the WlBuffer if the draw
    /// closure succeeds.  Otherwise, the buffer is freed immediately instead of waiting for a
    /// Release event that will never be sent if the WlBuffer is not used.
    pub fn try_draw<F, E>(
        &mut self,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
        draw: F,
    ) -> Result<wl_buffer::WlBuffer, E>
    where
        F: FnOnce(&mut [u8]) -> Result<(), E>,
        E: From<io::Error>,
    {
        let len = (height as usize) * (stride as usize);
        let alloc_len = (len + self.align - 1) & !(self.align - 1);
        let offset = self.alloc(alloc_len)?;
        let offset_i = offset as i32;
        if let Err(e) = draw(&mut self.inner.mmap[offset..][..len]) {
            Self::free(&self.free_list, offset, alloc_len);
            return Err(e);
        }
        let buffer = self.inner.pool.create_buffer(offset_i, width, height, stride, format);
        let free_list = self.free_list.clone();
        buffer.quick_assign(move |buffer, event, _| match event {
            wl_buffer::Event::Release => {
                buffer.destroy();
                Self::free(&free_list, offset, alloc_len);
            }
            _ => unreachable!(),
        });
        Ok(buffer.detach())
    }
}

fn create_shm_fd() -> io::Result<RawFd> {
    // Only try memfd on linux
    #[cfg(target_os = "linux")]
    loop {
        match memfd::memfd_create(
            CStr::from_bytes_with_nul(b"smithay-client-toolkit\0").unwrap(),
            memfd::MemFdCreateFlag::MFD_CLOEXEC | memfd::MemFdCreateFlag::MFD_ALLOW_SEALING,
        ) {
            Ok(fd) => {
                // this is only an optimization, so ignore errors
                let _ = fcntl::fcntl(
                    fd,
                    fcntl::F_ADD_SEALS(
                        fcntl::SealFlag::F_SEAL_SHRINK | fcntl::SealFlag::F_SEAL_SEAL,
                    ),
                );
                return Ok(fd);
            }
            Err(Errno::EINTR) => continue,
            Err(Errno::ENOSYS) => break,
            Err(errno) => return Err(errno.into()),
        }
    }

    // Fallback to using shm_open
    let sys_time = SystemTime::now();
    let mut mem_file_handle = format!(
        "/smithay-client-toolkit-{}",
        sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
    );
    loop {
        match mman::shm_open(
            mem_file_handle.as_str(),
            fcntl::OFlag::O_CREAT
                | fcntl::OFlag::O_EXCL
                | fcntl::OFlag::O_RDWR
                | fcntl::OFlag::O_CLOEXEC,
            stat::Mode::S_IRUSR | stat::Mode::S_IWUSR,
        ) {
            Ok(fd) => match mman::shm_unlink(mem_file_handle.as_str()) {
                Ok(_) => return Ok(fd),
                Err(errno) => match unistd::close(fd) {
                    Ok(_) => return Err(errno.into()),
                    Err(errno) => return Err(errno.into()),
                },
            },
            Err(Errno::EEXIST) => {
                // If a file with that handle exists then change the handle
                mem_file_handle = format!(
                    "/smithay-client-toolkit-{}",
                    sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
                );
                continue;
            }
            Err(Errno::EINTR) => continue,
            Err(errno) => return Err(errno.into()),
        }
    }
}

impl<E> crate::environment::Environment<E>
where
    E: crate::environment::GlobalHandler<wl_shm::WlShm>,
{
    /// Create a simple memory pool
    ///
    /// This memory pool track the usage of the buffers created from it,
    /// and invokes your callback when the compositor has finished using
    /// all of them.
    pub fn create_simple_pool<F>(&self, callback: F) -> io::Result<MemPool>
    where
        F: FnMut(wayland_client::DispatchData) + 'static,
    {
        MemPool::new(self.require_global::<wl_shm::WlShm>(), callback)
    }

    /// Create a double memory pool
    ///
    /// This can be used for double-buffered drawing. The memory pool
    /// is backed by two different SHM segments, which are used in alternance.
    ///
    /// The provided callback is triggered when one of the pools becomes unused again
    /// after you tried to draw while both where in use.
    pub fn create_double_pool<F>(&self, callback: F) -> io::Result<DoubleMemPool>
    where
        F: FnMut(wayland_client::DispatchData) + 'static,
    {
        DoubleMemPool::new(self.require_global::<wl_shm::WlShm>(), callback)
    }

    /// Create an automatic memory pool
    ///
    /// This pool will allocate more memory as needed in order to satisfy buffer requests, and will
    /// return memory to the pool when the compositor has finished using the memory.
    pub fn create_auto_pool(&self) -> io::Result<AutoMemPool> {
        AutoMemPool::new(self.require_global::<wl_shm::WlShm>())
    }
}
