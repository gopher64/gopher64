use sctk::data_device::{DataDevice, DataDeviceHandler, DataDeviceHandling, DndEvent};
use sctk::primary_selection::{
    PrimarySelectionDevice, PrimarySelectionDeviceManager, PrimarySelectionHandler,
    PrimarySelectionHandling,
};
use sctk::reexports::client::protocol::wl_seat::WlSeat;
use sctk::reexports::client::{Attached, DispatchData};
use sctk::seat::{SeatData, SeatHandler, SeatHandling, SeatListener};
use sctk::MissingGlobal;

/// Environemt setup for smithay-clipboard.
pub struct SmithayClipboard {
    seats: SeatHandler,
    primary_selection_manager: PrimarySelectionHandler,
    data_device_manager: DataDeviceHandler,
}

impl SmithayClipboard {
    /// Create new environment.
    pub fn new() -> Self {
        let mut seats = SeatHandler::new();
        let data_device_manager = DataDeviceHandler::init(&mut seats);
        let primary_selection_manager = PrimarySelectionHandler::init(&mut seats);
        Self { seats, primary_selection_manager, data_device_manager }
    }
}

// Seat handling for data device manager and primary selection.
impl SeatHandling for SmithayClipboard {
    fn listen<F: FnMut(Attached<WlSeat>, &SeatData, DispatchData) + 'static>(
        &mut self,
        f: F,
    ) -> SeatListener {
        self.seats.listen(f)
    }
}

impl PrimarySelectionHandling for SmithayClipboard {
    fn with_primary_selection<F: FnOnce(&PrimarySelectionDevice)>(
        &self,
        seat: &WlSeat,
        f: F,
    ) -> Result<(), MissingGlobal> {
        self.primary_selection_manager.with_primary_selection(seat, f)
    }

    fn get_primary_selection_manager(&self) -> Option<PrimarySelectionDeviceManager> {
        self.primary_selection_manager.get_primary_selection_manager()
    }
}

impl DataDeviceHandling for SmithayClipboard {
    fn set_callback<F>(&mut self, callback: F) -> Result<(), MissingGlobal>
    where
        F: FnMut(WlSeat, DndEvent, DispatchData) + 'static,
    {
        self.data_device_manager.set_callback(callback)
    }

    fn with_device<F: FnOnce(&DataDevice)>(
        &self,
        seat: &WlSeat,
        f: F,
    ) -> Result<(), MissingGlobal> {
        self.data_device_manager.with_device(seat, f)
    }
}

// Setup globals.
sctk::environment!(SmithayClipboard,
    singles = [
    sctk::reexports::protocols::unstable::primary_selection::v1::client::zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1 => primary_selection_manager,
    sctk::reexports::protocols::misc::gtk_primary_selection::client::gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager => primary_selection_manager,
    sctk::reexports::client::protocol::wl_data_device_manager::WlDataDeviceManager => data_device_manager,
    ],
multis = [
    WlSeat => seats,
]
);
