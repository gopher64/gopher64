use wayland_client::{
    protocol::{wl_data_device_manager, wl_data_source},
    Attached, DispatchData,
};

use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::{fs, io};

/// A data source for sending data though copy/paste or
/// drag and drop
#[derive(Debug)]
pub struct DataSource {
    pub(crate) source: wl_data_source::WlDataSource,
}

/// Possible events a data source needs to react to
#[derive(Debug)]
pub enum DataSourceEvent {
    /// Write the offered data for selected mime type
    ///
    /// This can happen several times during a dnd setup,
    /// and does not mean the action is finished.
    Send {
        /// Requested mime type
        mime_type: String,
        /// Pipe to write into
        pipe: WritePipe,
    },
    /// Target mime type
    ///
    /// Notifies that the target accepted a given mime type.
    /// You can use it to provide feedback (changing the icon
    /// of the drag'n'drop for example).
    ///
    /// Can be `None` if the current target does not accept any of the
    /// proposed mime types.
    ///
    /// This event can be emitted several times during the process
    Target {
        /// The type accepted by the target
        mime_type: Option<String>,
    },
    /// Notifies of the current selected action for the drag'n'drop
    ///
    /// Can only happen for data sources used during a drag'n'drop.
    ///
    /// This can change several times, the last received defines which action
    /// should actually be taken.
    Action {
        /// The action chosen by the target
        action: wl_data_device_manager::DndAction,
    },
    /// The action using this data source was cancelled.
    ///
    /// Once this event is received, the `DataSource` can not be used any more,
    /// and you should drop it for cleanup.
    ///
    /// Happens if the user cancels the current drag'n'drop, or replaces the
    /// selection buffer.
    Cancelled,
    /// The user performed the "drop" during a drag'n'drop
    ///
    /// This does not mean the operation is finished (the operation can still
    /// be cancelled afterwards).
    ///
    /// You are not guaranteed to receive this event at some point, as the compositor
    /// may cancel the action before the user drops.
    ///
    /// This event can only be generated on sources used for drag'n'drop, not
    /// selection sources.
    Dropped,
    /// The action is finished, this data source will not be used any more
    ///
    /// If the selected drag'n'drop action was "move", you can now delete the
    /// underlying resource.
    ///
    /// This event can only be generated on sources used for drag'n'drop, not
    /// selection sources.
    Finished,
}

fn data_source_impl<Impl>(
    evt: wl_data_source::Event,
    source: &wl_data_source::WlDataSource,
    implem: &mut Impl,
    ddata: DispatchData,
) where
    Impl: FnMut(DataSourceEvent, DispatchData),
{
    use self::wl_data_source::Event;
    let event = match evt {
        Event::Target { mime_type } => DataSourceEvent::Target { mime_type },
        Event::Send { mime_type, fd } => {
            DataSourceEvent::Send { mime_type, pipe: unsafe { FromRawFd::from_raw_fd(fd) } }
        }
        Event::Action { dnd_action } => DataSourceEvent::Action { action: dnd_action },
        Event::Cancelled => {
            source.destroy();
            DataSourceEvent::Cancelled
        }
        Event::DndDropPerformed => DataSourceEvent::Dropped,
        Event::DndFinished => {
            source.destroy();
            DataSourceEvent::Finished
        }
        _ => unreachable!(),
    };
    implem(event, ddata);
}

impl DataSource {
    /// Create a new data source
    ///
    /// You'll then need to provide it to a data device to send it
    /// either via selection (aka copy/paste) or via a drag and drop.
    pub fn new<F, S, It>(
        mgr: &Attached<wl_data_device_manager::WlDataDeviceManager>,
        mime_types: It,
        mut callback: F,
    ) -> DataSource
    where
        F: FnMut(DataSourceEvent, DispatchData) + 'static,
        S: Into<String>,
        It: IntoIterator<Item = S>,
    {
        let source = mgr.create_data_source();
        source.quick_assign(move |source, evt, dispatch_data| {
            data_source_impl(evt, &source, &mut callback, dispatch_data)
        });

        for mime in mime_types {
            source.offer(mime.into());
        }

        DataSource { source: source.detach() }
    }
}

/// A file descriptor that can only be written to
#[derive(Debug)]
pub struct WritePipe {
    file: fs::File,
}

impl io::Write for WritePipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl FromRawFd for WritePipe {
    unsafe fn from_raw_fd(fd: RawFd) -> WritePipe {
        WritePipe { file: FromRawFd::from_raw_fd(fd) }
    }
}

impl AsRawFd for WritePipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl IntoRawFd for WritePipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.into_raw_fd()
    }
}
