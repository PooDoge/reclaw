use std::{
    ffi::OsString,
    path::PathBuf,
    time::{Duration, SystemTime},
};

pub type AppId = u32;

#[derive(Clone, PartialEq, Debug)]
pub struct LaunchSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub env: Vec<(OsString, OsString)>,
    /// Variables the app must not inherit from the launcher (see `hostenv`).
    pub env_remove: Vec<OsString>,
    pub cwd: Option<PathBuf>,
    /// Append the app's stdout and stderr here. Without it they are inherited.
    pub log: Option<PathBuf>,
}

impl LaunchSpec {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self { program: program.into(), args: Vec::new(), env: Vec::new(), env_remove: Vec::new(), cwd: None, log: None }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }
}

/// What the UI shows for an app's run state. The supervisor reports `Running`, `Stopping` and
/// `Idle`; `Starting` exists for the UI to set optimistically between the press and the event.
#[derive(Clone, PartialEq, Debug, Default)]
pub enum RunState {
    #[default]
    Idle,
    Starting,
    Running {
        pid: u32,
        since: SystemTime,
    },
    Stopping {
        pid: u32,
    },
    /// The last session ended badly; cleared when the app is launched again.
    Failed(Outcome),
}

impl RunState {
    /// True while the app owns the gamepad and the screen.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Starting | Self::Running { .. } | Self::Stopping { .. })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Outcome {
    /// Exited with status 0.
    Exited { ran_for: Duration },
    /// Exited with a non-zero status. Many games do this on a normal quit, so it is not "crashed".
    ExitedWithCode { code: i32, ran_for: Duration },
    /// Killed by a signal nobody on our side sent.
    Signaled { signal: i32, ran_for: Duration },
    /// The user pressed Stop (or Stop twice, to force).
    Stopped { ran_for: Duration },
}

impl Outcome {
    /// Whether the UI should flag this ending (offer the log) rather than silently go idle.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::ExitedWithCode { .. } | Self::Signaled { .. })
    }

    pub fn ran_for(&self) -> Duration {
        match self {
            Self::Exited { ran_for }
            | Self::ExitedWithCode { ran_for, .. }
            | Self::Signaled { ran_for, .. }
            | Self::Stopped { ran_for } => *ran_for,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum SessionEvent {
    Started {
        app: AppId,
        pid: u32,
    },
    /// Sent once per session, after the whole process group is gone or reaped.
    Ended {
        app: AppId,
        outcome: Outcome,
    },
}
