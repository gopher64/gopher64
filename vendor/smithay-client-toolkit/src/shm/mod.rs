//! Various small utilities helping you to write clients

use std::{cell::RefCell, rc::Rc};

use wayland_client::{
    protocol::{wl_registry, wl_shm},
    Attached, DispatchData,
};

mod mempool;

pub use self::mempool::{AutoMemPool, DoubleMemPool, MemPool};
pub use wl_shm::Format;

/// A handler for the `wl_shm` global
///
/// This handler is automatically included in the
/// [`default_environment!`](../macro.default_environment.html).
#[derive(Debug)]
pub struct ShmHandler {
    shm: Option<Attached<wl_shm::WlShm>>,
    formats: Rc<RefCell<Vec<wl_shm::Format>>>,
}

impl ShmHandler {
    /// Create a new ShmHandler
    pub fn new() -> ShmHandler {
        ShmHandler { shm: None, formats: Rc::new(RefCell::new(vec![])) }
    }
}

impl crate::environment::GlobalHandler<wl_shm::WlShm> for ShmHandler {
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        _version: u32,
        _: DispatchData,
    ) {
        // only shm verison 1 is supported
        let shm = registry.bind::<wl_shm::WlShm>(1, id);
        let my_formats = self.formats.clone();
        shm.quick_assign(move |_, event, _| match event {
            wl_shm::Event::Format { format } => {
                my_formats.borrow_mut().push(format);
            }
            _ => unreachable!(),
        });
        self.shm = Some((*shm).clone());
    }
    fn get(&self) -> Option<Attached<wl_shm::WlShm>> {
        self.shm.clone()
    }
}

/// An interface trait to forward the shm handler capability
///
/// You need to implement this trait for you environment struct, by
/// delegating it to its `ShmHandler` field in order to get the
/// associated methods on your [`Environment`](../environment/struct.environment.html).
pub trait ShmHandling {
    /// Access the list of SHM formats supported by the compositor
    fn shm_formats(&self) -> Vec<wl_shm::Format>;
}

impl ShmHandling for ShmHandler {
    fn shm_formats(&self) -> Vec<wl_shm::Format> {
        self.formats.borrow().clone()
    }
}

impl<E> crate::environment::Environment<E>
where
    E: ShmHandling,
{
    /// Access the list of SHM formats supported by the compositor
    pub fn shm_formats(&self) -> Vec<wl_shm::Format> {
        self.with_inner(|inner| inner.shm_formats())
    }
}
