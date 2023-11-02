use wayland_protocols::unstable::primary_selection::v1::client::zwp_primary_selection_source_v1::{
    self, ZwpPrimarySelectionSourceV1,
};

use wayland_protocols::misc::gtk_primary_selection::client::gtk_primary_selection_source::{
    self, GtkPrimarySelectionSource,
};

use crate::data_device::WritePipe;

use std::os::unix::io::FromRawFd;

use wayland_client::DispatchData;

use super::PrimarySelectionDeviceManager;

/// A primary selection source for sending data through copy/paste.
#[derive(Debug)]
pub struct PrimarySelectionSource {
    pub(crate) source: PrimarySelectionSourceImpl,
}

/// Possible events a primary selection source needs to react to.
#[derive(Debug)]
pub enum PrimarySelectionSourceEvent {
    /// Write the offered data for selected mime type.
    Send {
        /// Requested mime type.
        mime_type: String,
        /// Pipe to write into.
        pipe: WritePipe,
    },

    /// The action using the primary selection source was cancelled.
    ///
    /// Once this event is received, the `PrimarySelectionSource` can not be used any more,
    /// and you should drop it for cleanup.
    ///
    /// Happens if the user replaces primary selection buffer.
    Cancelled,
}

impl PrimarySelectionSource {
    /// Create a new primary selection source.
    ///
    /// You'll then need to provide a primary selection device to send via selection.
    pub fn new<F, S, It>(
        manager: &PrimarySelectionDeviceManager,
        mime_types: It,
        mut callback: F,
    ) -> Self
    where
        F: FnMut(PrimarySelectionSourceEvent, DispatchData) + 'static,
        S: Into<String>,
        It: IntoIterator<Item = S>,
    {
        match manager {
            PrimarySelectionDeviceManager::Zwp(ref manager) => {
                let source = manager.create_source();
                source.quick_assign(move |source, event, dispatch_data| {
                    zwp_primary_source_imp(&source, event, dispatch_data, &mut callback);
                });

                for mime in mime_types {
                    source.offer(mime.into());
                }

                Self { source: PrimarySelectionSourceImpl::Zwp(source.detach()) }
            }
            PrimarySelectionDeviceManager::Gtk(ref manager) => {
                let source = manager.create_source();
                source.quick_assign(move |source, event, dispatch_data| {
                    gtk_primary_source_imp(&source, event, dispatch_data, &mut callback);
                });

                for mime in mime_types {
                    source.offer(mime.into());
                }

                Self { source: PrimarySelectionSourceImpl::Gtk(source.detach()) }
            }
        }
    }
}

/// Possible supported primary selection sources.
#[derive(Debug)]
pub(crate) enum PrimarySelectionSourceImpl {
    Zwp(ZwpPrimarySelectionSourceV1),
    Gtk(GtkPrimarySelectionSource),
}

fn gtk_primary_source_imp<Impl>(
    source: &GtkPrimarySelectionSource,
    event: gtk_primary_selection_source::Event,
    dispatch_data: DispatchData,
    implem: &mut Impl,
) where
    Impl: FnMut(PrimarySelectionSourceEvent, DispatchData),
{
    use gtk_primary_selection_source::Event;
    let event = match event {
        Event::Send { mime_type, fd } => PrimarySelectionSourceEvent::Send {
            mime_type,
            pipe: unsafe { FromRawFd::from_raw_fd(fd) },
        },
        Event::Cancelled => {
            source.destroy();
            PrimarySelectionSourceEvent::Cancelled
        }
        _ => unreachable!(),
    };

    implem(event, dispatch_data);
}

fn zwp_primary_source_imp<Impl>(
    source: &ZwpPrimarySelectionSourceV1,
    event: zwp_primary_selection_source_v1::Event,
    dispatch_data: DispatchData,
    implem: &mut Impl,
) where
    Impl: FnMut(PrimarySelectionSourceEvent, DispatchData),
{
    use zwp_primary_selection_source_v1::Event;
    let event = match event {
        Event::Send { mime_type, fd } => PrimarySelectionSourceEvent::Send {
            mime_type,
            pipe: unsafe { FromRawFd::from_raw_fd(fd) },
        },
        Event::Cancelled => {
            source.destroy();
            PrimarySelectionSourceEvent::Cancelled
        }
        _ => unreachable!(),
    };
    implem(event, dispatch_data);
}
