//! Set threads to realtime.
//!
//! Wrapper of the DBus interface: [`org.freedesktop.portal.Realtime`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Realtime).

use crate::{proxy::Proxy, Error};

/// Interface for setting a thread to realtime from within the sandbox.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.Realtime`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.Realtime).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.Realtime")]
pub struct Realtime<'a>(Proxy<'a>);

impl<'a> Realtime<'a> {
    /// Create a new instance of [`Realtime`].
    pub async fn new() -> Result<Realtime<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.Realtime").await?;
        Ok(Self(proxy))
    }

    #[doc(alias = "MakeThreadRealtimeWithPID")]
    #[allow(missing_docs)]
    pub async fn max_thread_realtime_with_pid(
        &self,
        process: u64,
        thread: u64,
        priority: u32,
    ) -> Result<(), Error> {
        self.0
            .call("MakeThreadRealtimeWithPID", &(&process, &thread, &priority))
            .await
    }

    #[doc(alias = "MakeThreadHighPriorityWithPID")]
    #[allow(missing_docs)]
    pub async fn max_thread_high_priority_with_pid(
        &self,
        process: u64,
        thread: u64,
        priority: i32,
    ) -> Result<(), Error> {
        self.0
            .call(
                "MakeThreadHighPriorityWithPID",
                &(&process, &thread, &priority),
            )
            .await
    }

    #[doc(alias = "MaxRealtimePriority")]
    #[allow(missing_docs)]
    pub async fn max_realtime_priority(&self) -> Result<i64, Error> {
        self.0.property("MaxRealtimePriority").await
    }

    #[doc(alias = "MinNiceLevel")]
    #[allow(missing_docs)]
    pub async fn min_nice_level(&self) -> Result<u32, Error> {
        self.0.property("MinNiceLevel").await
    }

    #[doc(alias = "RTTimeUSecMax")]
    #[allow(missing_docs)]
    pub async fn rt_time_usec_max(&self) -> Result<u32, Error> {
        self.0.property("RTTimeUSecMax").await
    }
}
