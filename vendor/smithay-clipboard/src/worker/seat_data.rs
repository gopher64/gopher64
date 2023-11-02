use sctk::reexports::client::protocol::wl_keyboard::WlKeyboard;
use sctk::reexports::client::protocol::wl_pointer::WlPointer;
use sctk::reexports::client::protocol::wl_seat::WlSeat;

/// Data to track seat capability changes and handle release of the objects.
pub struct SeatData {
    pub seat: WlSeat,
    pub keyboard: Option<WlKeyboard>,
    pub pointer: Option<WlPointer>,
}

impl SeatData {
    pub fn new(seat: WlSeat, keyboard: Option<WlKeyboard>, pointer: Option<WlPointer>) -> Self {
        SeatData { seat, keyboard, pointer }
    }
}
