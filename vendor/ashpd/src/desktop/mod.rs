mod handle_token;
pub(crate) mod request;
mod session;
pub(crate) use self::handle_token::HandleToken;
pub use self::{
    request::{Request, Response, ResponseError},
    session::Session,
};
mod icon;
pub use icon::Icon;

pub mod account;
pub mod background;
pub mod camera;
pub mod clipboard;
pub mod device;
pub mod dynamic_launcher;
pub mod email;
/// Open/save file(s) chooser.
pub mod file_chooser;
/// Enable/disable/query the status of Game Mode.
pub mod game_mode;
/// Register global shortcuts
pub mod global_shortcuts;
/// Inhibit the session from being restarted or the user from logging out.
pub mod inhibit;
/// Capture input events from physical or logical devices.
pub mod input_capture;
/// Query the user's GPS location.
pub mod location;
/// Monitor memory level.
pub mod memory_monitor;
/// Check the status of the network on a user's machine.
pub mod network_monitor;
/// Send/withdraw notifications.
pub mod notification;
pub mod open_uri;
/// Power profile monitoring.
pub mod power_profile_monitor;
/// Print a document.
pub mod print;
/// Proxy information.
pub mod proxy_resolver;
pub mod realtime;
/// Start a remote desktop session and interact with it.
pub mod remote_desktop;
pub mod screencast;
pub mod screenshot;
/// Retrieve a per-application secret used to encrypt confidential data inside
/// the sandbox.
pub mod secret;
/// Read & listen to system settings changes.
pub mod settings;
pub mod trash;
pub mod wallpaper;
