//! # Examples
//!
//! ```rust,no_run
//! use ashpd::desktop::game_mode::GameMode;
//!
//! async fn run() -> ashpd::Result<()> {
//!     let proxy = GameMode::new().await?;
//!
//!     println!("{:#?}", proxy.register(246612).await?);
//!     println!("{:#?}", proxy.query_status(246612).await?);
//!     println!("{:#?}", proxy.unregister(246612).await?);
//!     println!("{:#?}", proxy.query_status(246612).await?);
//!
//!     Ok(())
//! }
//! ```

use std::{fmt::Debug, os::unix::io::AsRawFd};

use serde_repr::Deserialize_repr;
use zbus::zvariant::{Fd, Type};

use crate::{error::PortalError, proxy::Proxy, Error};

#[derive(Deserialize_repr, PartialEq, Eq, Debug, Type)]
/// The status of the game mode.
#[repr(i32)]
pub enum Status {
    /// GameMode is inactive.
    Inactive = 0,
    /// GameMode is active.
    Active = 1,
    /// GameMode is active and `pid` is registered.
    Registered = 2,
    /// The query failed inside GameMode.
    Rejected = -1,
}

#[derive(Deserialize_repr, PartialEq, Eq, Debug, Type)]
#[repr(i32)]
/// The status of a (un-)register game mode request.
enum RegisterStatus {
    /// If the game was successfully (un-)registered.
    Success = 0,
    /// If the request was rejected by GameMode.
    Rejected = -1,
}

/// The interface lets sandboxed applications access GameMode from within the
/// sandbox.
///
/// It is analogous to the `com.feralinteractive.GameMode` interface and will
/// proxy request there, but with additional permission checking and pid
/// mapping. The latter is necessary in the case that sandbox has pid namespace
/// isolation enabled. See the man page for pid_namespaces(7) for more details,
/// but briefly, it means that the sandbox has its own process id namespace
/// which is separated from the one on the host. Thus there will be two separate
/// process ids (pids) within two different namespaces that both identify same
/// process. One id from the pid namespace inside the sandbox and one id from
/// the host pid namespace. Since GameMode expects pids from the host pid
/// namespace but programs inside the sandbox can only know pids from the
/// sandbox namespace, process ids need to be translated from the portal to the
/// host namespace. The portal will do that transparently for all calls where
/// this is necessary.
///
/// Note: GameMode will monitor active clients, i.e. games and other programs
/// that have successfully called [`GameMode::register`]. In the event
/// that a client terminates without a call to the
/// [`GameMode::unregister`] method, GameMode will automatically
/// un-register the client. This might happen with a (small) delay.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.GameMode`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.GameMode).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.GameMode")]
pub struct GameMode<'a>(Proxy<'a>);

impl<'a> GameMode<'a> {
    /// Create a new instance of [`GameMode`].
    pub async fn new() -> Result<GameMode<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.GameMode").await?;
        Ok(Self(proxy))
    }

    /// Query the GameMode status for a process.
    /// If the caller is running inside a sandbox with pid namespace isolation,
    /// the pid will be translated to the respective host pid.
    ///
    /// # Arguments
    ///
    /// * `pid` - Process id to query the GameMode status of.
    ///
    /// # Specifications
    ///
    /// See also [`QueryStatus`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GameMode.QueryStatus).
    #[doc(alias = "QueryStatus")]
    pub async fn query_status(&self, pid: u32) -> Result<Status, Error> {
        self.0.call("QueryStatus", &(pid)).await
    }

    /// Query the GameMode status for a process.
    ///
    /// # Arguments
    ///
    /// * `target` - Pid file descriptor to query the GameMode status of.
    /// * `requester` - Pid file descriptor of the process requesting the
    ///   information.
    ///
    /// # Specifications
    ///
    /// See also [`QueryStatusByPIDFd`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GameMode.QueryStatusByPIDFd).
    #[doc(alias = "QueryStatusByPIDFd")]
    pub async fn query_status_by_pidfd(
        &self,
        target: &impl AsRawFd,
        requester: &impl AsRawFd,
    ) -> Result<Status, Error> {
        self.0
            .call(
                "QueryStatusByPIDFd",
                &(
                    Fd::from(target.as_raw_fd()),
                    Fd::from(requester.as_raw_fd()),
                ),
            )
            .await
    }

    /// Query the GameMode status for a process.
    ///
    /// # Arguments
    ///
    /// * `target` - Process id to query the GameMode status of.
    /// * `requester` - Process id of the process requesting the information.
    ///
    /// # Specifications
    ///
    /// See also [`QueryStatusByPid`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GameMode.QueryStatusByPid).
    #[doc(alias = "QueryStatusByPid")]
    pub async fn query_status_by_pid(&self, target: u32, requester: u32) -> Result<Status, Error> {
        self.0.call("QueryStatusByPid", &(target, requester)).await
    }

    /// Register a game with GameMode and thus request GameMode to be activated.
    /// If the caller is running inside a sandbox with pid namespace isolation,
    /// the pid will be translated to the respective host pid. See the general
    /// introduction for details. If the GameMode has already been requested
    /// for pid before, this call will fail.
    ///
    /// # Arguments
    ///
    /// * `pid` - Process id of the game to register.
    ///
    /// # Specifications
    ///
    /// See also [`RegisterGame`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GameMode.RegisterGame).
    #[doc(alias = "RegisterGame")]
    pub async fn register(&self, pid: u32) -> Result<(), Error> {
        let status = self.0.call("RegisterGame", &(pid)).await?;
        match status {
            RegisterStatus::Success => Ok(()),
            RegisterStatus::Rejected => Err(Error::Portal(PortalError::Failed)),
        }
    }

    /// Register a game with GameMode.
    ///
    /// # Arguments
    ///
    /// * `target` - Process file descriptor of the game to register.
    /// * `requester` - Process file descriptor of the process requesting the
    ///   registration.
    ///
    /// # Specifications
    ///
    /// See also [`RegisterGameByPIDFd`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GameMode.RegisterGameByPIDFd).
    #[doc(alias = "RegisterGameByPIDFd")]
    pub async fn register_by_pidfd(
        &self,
        target: &impl AsRawFd,
        requester: &impl AsRawFd,
    ) -> Result<(), Error> {
        let status = self
            .0
            .call(
                "RegisterGameByPIDFd",
                &(
                    Fd::from(target.as_raw_fd()),
                    Fd::from(requester.as_raw_fd()),
                ),
            )
            .await?;
        match status {
            RegisterStatus::Success => Ok(()),
            RegisterStatus::Rejected => Err(Error::Portal(PortalError::Failed)),
        }
    }

    /// Register a game with GameMode.
    ///
    /// # Arguments
    ///
    /// * `target` - Process id of the game to register.
    /// * `requester` - Process id of the process requesting the registration.
    ///
    /// # Specifications
    ///
    /// See also [`RegisterGameByPid`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GameMode.RegisterGameByPid).
    #[doc(alias = "RegisterGameByPid")]
    pub async fn register_by_pid(&self, target: u32, requester: u32) -> Result<(), Error> {
        let status = self
            .0
            .call("RegisterGameByPid", &(target, requester))
            .await?;
        match status {
            RegisterStatus::Success => Ok(()),
            RegisterStatus::Rejected => Err(Error::Portal(PortalError::Failed)),
        }
    }

    /// Un-register a game from GameMode.
    /// if the call is successful and there are no other games or clients
    /// registered, GameMode will be deactivated. If the caller is running
    /// inside a sandbox with pid namespace isolation, the pid will be
    /// translated to the respective host pid.
    ///
    /// # Arguments
    ///
    /// * `pid` - Process id of the game to un-register.
    ///
    /// # Specifications
    ///
    /// See also [`UnregisterGame`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GameMode.UnregisterGame).
    #[doc(alias = "UnregisterGame")]
    pub async fn unregister(&self, pid: u32) -> Result<(), Error> {
        let status = self.0.call("UnregisterGame", &(pid)).await?;
        match status {
            RegisterStatus::Success => Ok(()),
            RegisterStatus::Rejected => Err(Error::Portal(PortalError::Failed)),
        }
    }

    /// Un-register a game from GameMode.
    ///
    /// # Arguments
    ///
    /// * `target` - Pid file descriptor of the game to un-register.
    /// * `requester` - Pid file descriptor of the process requesting the
    ///   un-registration.
    ///
    /// # Specifications
    ///
    /// See also [`UnregisterGameByPIDFd`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GameMode.UnregisterGameByPIDFd).
    #[doc(alias = "UnregisterGameByPIDFd")]
    pub async fn unregister_by_pidfd(
        &self,
        target: &impl AsRawFd,
        requester: &impl AsRawFd,
    ) -> Result<(), Error> {
        let status = self
            .0
            .call(
                "UnregisterGameByPIDFd",
                &(
                    Fd::from(target.as_raw_fd()),
                    Fd::from(requester.as_raw_fd()),
                ),
            )
            .await?;
        match status {
            RegisterStatus::Success => Ok(()),
            RegisterStatus::Rejected => Err(Error::Portal(PortalError::Failed)),
        }
    }

    /// Un-register a game from GameMode.
    ///
    /// # Arguments
    ///
    /// * `target` - Process id of the game to un-register.
    /// * `requester` - Process id of the process requesting the
    ///   un-registration.
    ///
    /// # Specifications
    ///
    /// See also [`UnregisterGameByPid`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-GameMode.UnregisterGameByPid).
    #[doc(alias = "UnregisterGameByPid")]
    pub async fn unregister_by_pid(&self, target: u32, requester: u32) -> Result<(), Error> {
        let status = self
            .0
            .call("UnregisterGameByPid", &(target, requester))
            .await?;
        match status {
            RegisterStatus::Success => Ok(()),
            RegisterStatus::Rejected => Err(Error::Portal(PortalError::Failed)),
        }
    }
}
