use std::collections::VecDeque;
use std::slice::IterMut;

use sctk::reexports::client::protocol::wl_seat::WlSeat;

use super::seat_data::SeatData;

/// Data to track latest seat and serial for clipboard requests.
pub struct ClipboardDispatchData {
    /// Seats that our application encountered. The first seat is the latest one we've encountered.
    observed_seats: VecDeque<(WlSeat, u32)>,

    /// All the seats that were advertised.
    seats: Vec<SeatData>,
}

impl ClipboardDispatchData {
    /// Builds new `ClipboardDispatchData` with all fields equal to `None`.
    pub fn new(seats: Vec<SeatData>) -> Self {
        Self { observed_seats: Default::default(), seats }
    }

    /// Returns the requested seat's data or adds a new one.
    pub fn get_seat_data_or_add(&mut self, seat: WlSeat) -> &mut SeatData {
        let pos = self.seats.iter().position(|st| st.seat == seat);
        let index = pos.unwrap_or_else(|| {
            self.seats.push(SeatData::new(seat, None, None));
            self.seats.len() - 1
        });

        &mut self.seats[index]
    }

    pub fn seats(&mut self) -> IterMut<'_, SeatData> {
        self.seats.iter_mut()
    }

    /// Set the last observed seat.
    pub fn set_last_observed_seat(&mut self, seat: WlSeat, serial: u32) {
        // Assure each seat exists only once.
        self.remove_observed_seat(&seat);

        // Add the seat to front, making it the latest observed one.
        self.observed_seats.push_front((seat, serial));
    }

    /// Remove the given seat from the observed seats.
    pub fn remove_observed_seat(&mut self, seat: &WlSeat) {
        if let Some(pos) = self.observed_seats.iter().position(|st| &st.0 == seat) {
            self.observed_seats.remove(pos);
        }
    }

    /// Return the last observed seat and the serial.
    pub fn last_observed_seat(&self) -> Option<&(WlSeat, u32)> {
        self.observed_seats.front()
    }
}
