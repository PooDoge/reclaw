//! Launching and supervising apps: Play becomes Stop.
//!
//! * [`Supervisor`] spawns an app in its **own process group**, so wrapper scripts and the real
//!   game die together, and stops it gracefully (SIGTERM) with escalation to SIGKILL.
//! * [`InputProfile`] hands the launched app the controller setup the user chose in Reclaw, for
//!   apps that read SDL's `SDL_GAMECONTROLLERCONFIG`.
mod profile;
mod state;
mod supervisor;

pub use profile::{InputProfile, SdlMapping};
pub use state::{AppId, LaunchSpec, Outcome, RunState, SessionEvent};
pub use supervisor::{LaunchError, StopError, Supervisor};
