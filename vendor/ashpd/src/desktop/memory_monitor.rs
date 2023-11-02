//! # Examples
//!
//! ```rust,no_run
//! use ashpd::desktop::memory_monitor::MemoryMonitor;
//! use futures_util::StreamExt;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let proxy = MemoryMonitor::new().await?;
//!     let level = proxy
//!         .receive_low_memory_warning()
//!         .await?
//!         .next()
//!         .await
//!         .expect("Stream exhausted");
//!     println!("{}", level);
//!     Ok(())
//! }
//! ```

use futures_util::Stream;

use crate::{proxy::Proxy, Error};

/// The interface provides information about low system memory to sandboxed
/// applications.
///
/// It is not a portal in the strict sense, since it does not involve user
/// interaction.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.MemoryMonitor`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.MemoryMonitor).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.MemoryMonitor")]
pub struct MemoryMonitor<'a>(Proxy<'a>);

impl<'a> MemoryMonitor<'a> {
    /// Create a new instance of [`MemoryMonitor`].
    pub async fn new() -> Result<MemoryMonitor<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.MemoryMonitor").await?;
        Ok(Self(proxy))
    }

    /// Signal emitted when a particular low memory situation happens
    /// with 0 being the lowest level of memory availability warning, and 255
    /// being the highest.
    ///
    /// # Specifications
    ///
    /// See also [`LowMemoryWarning`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-signal-org-freedesktop-portal-MemoryMonitor.LowMemoryWarning).
    #[doc(alias = "LowMemoryWarning")]
    pub async fn receive_low_memory_warning(&self) -> Result<impl Stream<Item = i32>, Error> {
        self.0.signal("LowMemoryWarning").await
    }
}
