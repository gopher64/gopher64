//! Re-implementation of [`std::time::SystemTime`].

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::time::Duration;

/// See [`std::time::SystemTime`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SystemTime(i64);

impl SystemTime {
	/// See [`std::time::SystemTime::UNIX_EPOCH`].
	pub const UNIX_EPOCH: Self = Self(0);

	/// See [`std::time::SystemTime::now()`].
	#[must_use]
	pub fn now() -> Self {
		#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
		Self(js_sys::Date::now() as i64)
	}

	/// See [`std::time::SystemTime::duration_since()`].
	#[allow(
		clippy::missing_errors_doc,
		clippy::missing_panics_doc,
		clippy::trivially_copy_pass_by_ref
	)]
	pub fn duration_since(&self, earlier: Self) -> Result<Duration, SystemTimeError> {
		if self.0 < earlier.0 {
			let duration = (earlier.0 - self.0)
				.try_into()
				.expect("`self` bigger then `earlier` despite earlier check");

			Err(SystemTimeError(Duration::from_millis(duration)))
		} else {
			let duration = (self.0 - earlier.0)
				.try_into()
				.expect("`earlier` bigger then `self` despite earlier check");
			Ok(Duration::from_millis(duration))
		}
	}

	/// See [`std::time::SystemTime::elapsed()`].
	#[allow(clippy::missing_errors_doc, clippy::trivially_copy_pass_by_ref)]
	pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
		Self::now().duration_since(*self)
	}

	/// See [`std::time::SystemTime::checked_add()`].
	#[allow(clippy::trivially_copy_pass_by_ref)]
	pub fn checked_add(&self, duration: Duration) -> Option<Self> {
		let duration = duration.as_millis().try_into().ok()?;
		self.0.checked_add(duration).map(SystemTime)
	}

	/// See [`std::time::SystemTime::checked_sub()`].
	#[allow(clippy::trivially_copy_pass_by_ref)]
	pub fn checked_sub(&self, duration: Duration) -> Option<Self> {
		let duration = duration.as_millis().try_into().ok()?;
		self.0.checked_sub(duration).map(SystemTime)
	}
}

impl Add<Duration> for SystemTime {
	type Output = Self;

	/// # Panics
	///
	/// This function may panic if the resulting point in time cannot be
	/// represented by the underlying data structure. See
	/// [`SystemTime::checked_add`] for a version without panic.
	fn add(self, dur: Duration) -> Self {
		self.checked_add(dur)
			.expect("overflow when adding duration to instant")
	}
}

impl AddAssign<Duration> for SystemTime {
	fn add_assign(&mut self, other: Duration) {
		*self = *self + other;
	}
}

impl Sub<Duration> for SystemTime {
	type Output = Self;

	fn sub(self, dur: Duration) -> Self {
		self.checked_sub(dur)
			.expect("overflow when subtracting duration from instant")
	}
}

impl SubAssign<Duration> for SystemTime {
	fn sub_assign(&mut self, other: Duration) {
		*self = *self - other;
	}
}

/// See [`std::time::SystemTimeError`].
#[derive(Clone, Debug)]
#[allow(missing_copy_implementations)]
pub struct SystemTimeError(Duration);

impl SystemTimeError {
	/// See [`std::time::SystemTimeError::duration()`].
	#[must_use]
	#[allow(clippy::missing_const_for_fn)]
	pub fn duration(&self) -> Duration {
		self.0
	}
}

impl Display for SystemTimeError {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		write!(formatter, "second time provided was later than self")
	}
}

impl Error for SystemTimeError {}
