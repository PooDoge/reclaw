//! Launching and supervising apps: Play becomes Stop.
//!
//! * [`Supervisor`] spawns an app in its **own process group**, so wrapper scripts and the real
//!   game die together, and stops it gracefully (SIGTERM) with escalation to SIGKILL.
//! * `runner`: Wine, Proton and custom commands for Windows programs on Linux; `hostenv`: the clean environment a game starts with
//! * [`InputProfile`] hands the launched app the controller setup the user chose in Reclaw, for
//!   apps that read SDL's `SDL_GAMECONTROLLERCONFIG`.
mod hostenv;
mod profile;
mod runner;
mod state;
mod supervisor;

pub use hostenv::{HOST_BREAKING, apply as clean_environment, clean_path};
pub use profile::{InputProfile, SdlMapping};
pub use runner::{
    Probe, RunnerCommand, RunnerConfig, RunnerError, RunnerKind, available as runner_available, command as runner_command, proton_installs,
    split_command,
};
pub use state::{AppId, LaunchSpec, Outcome, RunState, SessionEvent};
pub use supervisor::{LaunchError, StopError, Supervisor};
