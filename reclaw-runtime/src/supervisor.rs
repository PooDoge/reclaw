use std::{
    collections::HashMap,
    fs::OpenOptions,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};

use crate::state::{AppId, LaunchSpec, Outcome, RunState, SessionEvent};

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("app {0} is already running")]
    AlreadyRunning(AppId),
    #[error("could not open the log file: {0}")]
    Log(std::io::Error),
    #[error("could not start {program}: {source}")]
    Spawn { program: String, source: std::io::Error },
}

#[derive(Debug, thiserror::Error)]
pub enum StopError {
    #[error("app {0} is not running")]
    NotRunning(AppId),
}

struct Active {
    pid: u32,
    since: SystemTime,
    stop_requested: Arc<AtomicBool>,
    exited: Arc<AtomicBool>,
    stopping: bool,
}

type Table = Arc<Mutex<HashMap<AppId, Active>>>;

/// Runs at most one session per app. Cheap to clone; clones share the same sessions.
#[derive(Clone)]
pub struct Supervisor {
    sessions: Table,
    last_failure: Arc<Mutex<HashMap<AppId, Outcome>>>,
    events: UnboundedSender<SessionEvent>,
    grace: Duration,
}

impl Supervisor {
    /// `grace` is how long a graceful stop may take before the whole group is killed.
    pub fn new(grace: Duration) -> (Self, UnboundedReceiver<SessionEvent>) {
        let (events, rx) = unbounded();
        let this = Self { sessions: Arc::default(), last_failure: Arc::default(), events, grace };
        (this, rx)
    }

    pub fn state(&self, app: AppId) -> RunState {
        if let Some(a) = lock(&self.sessions).get(&app) {
            return if a.stopping { RunState::Stopping { pid: a.pid } } else { RunState::Running { pid: a.pid, since: a.since } };
        }
        lock(&self.last_failure).get(&app).cloned().map_or(RunState::Idle, RunState::Failed)
    }

    pub fn running(&self) -> Vec<AppId> {
        let mut apps: Vec<_> = lock(&self.sessions).keys().copied().collect();
        apps.sort_unstable();
        apps
    }

    pub fn start(&self, app: AppId, spec: LaunchSpec) -> Result<u32, LaunchError> {
        // Hold the table across spawn so two quick presses cannot launch two copies.
        let mut table = lock(&self.sessions);
        if table.contains_key(&app) {
            return Err(LaunchError::AlreadyRunning(app));
        }

        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args).envs(spec.env.iter().map(|(k, v)| (k, v))).stdin(Stdio::null());
        if let Some(cwd) = &spec.cwd {
            cmd.current_dir(cwd);
        }
        if let Some(path) = &spec.log {
            let file = OpenOptions::new().create(true).append(true).open(path).map_err(LaunchError::Log)?;
            let err = file.try_clone().map_err(LaunchError::Log)?;
            cmd.stdout(file).stderr(err);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // New process group with pgid == pid: one killpg reaches wrappers and children.
            cmd.process_group(0);
        }

        let mut child = cmd.spawn().map_err(|source| LaunchError::Spawn { program: spec.program.display().to_string(), source })?;
        let pid = child.id();
        let started = Instant::now();
        let since = SystemTime::now();
        let stop_requested = Arc::new(AtomicBool::new(false));
        let exited = Arc::new(AtomicBool::new(false));
        table.insert(app, Active { pid, since, stop_requested: stop_requested.clone(), exited: exited.clone(), stopping: false });
        drop(table);
        lock(&self.last_failure).remove(&app);
        let _ = self.events.unbounded_send(SessionEvent::Started { app, pid });

        let this = self.clone();
        thread::Builder::new()
            .name(format!("reclaw-app-{app}"))
            .spawn(move || {
                let status = child.wait();
                // Flag first, so a watchdog that wakes now does not signal a reaped pid.
                exited.store(true, Ordering::SeqCst);
                let ran_for = started.elapsed();
                let outcome = classify(status, stop_requested.load(Ordering::SeqCst), ran_for);
                lock(&this.sessions).remove(&app);
                if outcome.is_failure() {
                    lock(&this.last_failure).insert(app, outcome.clone());
                }
                let _ = this.events.unbounded_send(SessionEvent::Ended { app, outcome });
            })
            .map_err(|source| LaunchError::Spawn { program: spec.program.display().to_string(), source })?;
        Ok(pid)
    }

    /// First call asks the app to quit and starts a watchdog that force-kills after `grace`.
    /// A second call while it is still stopping kills immediately (Stop, Stop = force quit).
    pub fn stop(&self, app: AppId) -> Result<(), StopError> {
        let (pid, force, exited) = {
            let mut table = lock(&self.sessions);
            let a = table.get_mut(&app).ok_or(StopError::NotRunning(app))?;
            let force = a.stopping;
            a.stopping = true;
            a.stop_requested.store(true, Ordering::SeqCst);
            (a.pid, force, a.exited.clone())
        };
        if force {
            terminate(pid, true);
            return Ok(());
        }
        terminate(pid, false);
        let grace = self.grace;
        thread::spawn(move || {
            let deadline = Instant::now() + grace;
            while Instant::now() < deadline {
                if exited.load(Ordering::SeqCst) {
                    return;
                }
                thread::sleep(Duration::from_millis(20));
            }
            if !exited.load(Ordering::SeqCst) {
                terminate(pid, true);
            }
        });
        Ok(())
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // A panic elsewhere must not wedge the launcher: the tables hold plain data.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn classify(status: std::io::Result<std::process::ExitStatus>, stop_requested: bool, ran_for: Duration) -> Outcome {
    if stop_requested {
        return Outcome::Stopped { ran_for };
    }
    let Ok(status) = status else {
        return Outcome::ExitedWithCode { code: -1, ran_for };
    };
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return Outcome::Signaled { signal, ran_for };
        }
    }
    match status.code() {
        Some(0) => Outcome::Exited { ran_for },
        Some(code) => Outcome::ExitedWithCode { code, ran_for },
        None => Outcome::ExitedWithCode { code: -1, ran_for },
    }
}

#[cfg(unix)]
fn terminate(pid: u32, force: bool) {
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    // Negative result just means the group is already gone.
    unsafe {
        libc::killpg(pid as libc::pid_t, signal);
    }
}

/// Windows: `taskkill /T` walks the process tree; `/F` forces. Not exercised by tests here.
#[cfg(not(unix))]
fn terminate(pid: u32, force: bool) {
    let mut cmd = Command::new("taskkill");
    cmd.args(["/PID", &pid.to_string(), "/T"]);
    if force {
        cmd.arg("/F");
    }
    let _ = cmd.stdout(Stdio::null()).stderr(Stdio::null()).status();
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use futures_channel::mpsc::UnboundedReceiver;

    fn sh(script: &str) -> LaunchSpec {
        LaunchSpec::new("sh").arg("-c").arg(script)
    }

    /// Block until the next Ended event (with a ceiling so a bug fails instead of hanging).
    fn wait_ended(rx: &mut UnboundedReceiver<SessionEvent>) -> Outcome {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            match rx.try_recv() {
                Ok(SessionEvent::Ended { outcome, .. }) => return outcome,
                Ok(_) | Err(_) => thread::sleep(Duration::from_millis(10)),
            }
        }
        panic!("session did not end in time");
    }

    #[test]
    fn clean_exit() {
        let (sup, mut rx) = Supervisor::new(Duration::from_secs(1));
        sup.start(1, sh("exit 0")).unwrap();
        assert!(matches!(wait_ended(&mut rx), Outcome::Exited { .. }));
        assert_eq!(sup.state(1), RunState::Idle);
    }

    #[test]
    fn non_zero_exit_is_reported_and_remembered() {
        let (sup, mut rx) = Supervisor::new(Duration::from_secs(1));
        sup.start(1, sh("exit 3")).unwrap();
        let outcome = wait_ended(&mut rx);
        assert!(matches!(outcome, Outcome::ExitedWithCode { code: 3, .. }));
        assert_eq!(sup.state(1), RunState::Failed(outcome));
        // Launching again clears the remembered failure.
        sup.start(1, sh("exit 0")).unwrap();
        wait_ended(&mut rx);
        assert_eq!(sup.state(1), RunState::Idle);
    }

    #[test]
    fn spawn_failure_is_an_error_not_a_session() {
        let (sup, _rx) = Supervisor::new(Duration::from_secs(1));
        let err = sup.start(1, LaunchSpec::new("/definitely/not/here")).unwrap_err();
        assert!(matches!(err, LaunchError::Spawn { .. }));
        assert_eq!(sup.state(1), RunState::Idle);
        assert!(sup.running().is_empty());
    }

    #[test]
    fn second_launch_is_refused_while_running() {
        let (sup, mut rx) = Supervisor::new(Duration::from_secs(1));
        sup.start(1, sh("sleep 30")).unwrap();
        assert!(matches!(sup.start(1, sh("true")), Err(LaunchError::AlreadyRunning(1))));
        assert!(matches!(sup.state(1), RunState::Running { .. }));
        sup.stop(1).unwrap();
        wait_ended(&mut rx);
    }

    #[test]
    fn stop_is_graceful() {
        let (sup, mut rx) = Supervisor::new(Duration::from_secs(5));
        sup.start(1, sh("sleep 30")).unwrap();
        sup.stop(1).unwrap();
        assert!(matches!(sup.state(1), RunState::Stopping { .. }));
        let began = Instant::now();
        assert!(matches!(wait_ended(&mut rx), Outcome::Stopped { .. }));
        assert!(began.elapsed() < Duration::from_secs(3), "SIGTERM should not need the grace period");
    }

    #[test]
    fn stubborn_app_is_killed_after_the_grace_period() {
        let (sup, mut rx) = Supervisor::new(Duration::from_millis(300));
        // Ignores SIGTERM. `sleep` in the foreground is in the same group and is killed too.
        sup.start(1, sh("trap '' TERM; while true; do sleep 1; done")).unwrap();
        thread::sleep(Duration::from_millis(100));
        sup.stop(1).unwrap();
        let began = Instant::now();
        assert!(matches!(wait_ended(&mut rx), Outcome::Stopped { .. }));
        assert!(began.elapsed() >= Duration::from_millis(250), "must wait out the grace period");
        assert!(began.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn second_stop_forces() {
        let (sup, mut rx) = Supervisor::new(Duration::from_secs(60));
        sup.start(1, sh("trap '' TERM; while true; do sleep 1; done")).unwrap();
        thread::sleep(Duration::from_millis(100));
        sup.stop(1).unwrap();
        sup.stop(1).unwrap();
        let began = Instant::now();
        wait_ended(&mut rx);
        assert!(began.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn stop_kills_children_in_the_group() {
        let dir = std::env::temp_dir().join(format!("reclaw-rt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pidfile = dir.join("child.pid");
        let (sup, mut rx) = Supervisor::new(Duration::from_secs(2));
        sup.start(1, sh(&format!("sleep 60 & echo $! > {}; wait", pidfile.display()))).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let child_pid = loop {
            if let Some(pid) = std::fs::read_to_string(&pidfile).ok().and_then(|t| t.trim().parse::<i32>().ok()) {
                break pid;
            }
            assert!(Instant::now() < deadline, "child never wrote its pid");
            thread::sleep(Duration::from_millis(10));
        };
        sup.stop(1).unwrap();
        wait_ended(&mut rx);
        // Reaping of the orphaned grandchild by init is asynchronous; poll briefly.
        let gone = (0..100).any(|_| {
            thread::sleep(Duration::from_millis(20));
            unsafe { libc::kill(child_pid, 0) == -1 }
        });
        assert!(gone, "grandchild survived the stop");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn stopping_an_idle_app_is_an_error() {
        let (sup, _rx) = Supervisor::new(Duration::from_secs(1));
        assert!(matches!(sup.stop(9), Err(StopError::NotRunning(9))));
    }

    #[test]
    fn external_signal_is_a_failure_not_a_stop() {
        let (sup, mut rx) = Supervisor::new(Duration::from_secs(1));
        let pid = sup.start(1, sh("sleep 30")).unwrap();
        unsafe { libc::killpg(pid as i32, libc::SIGSEGV) };
        let outcome = wait_ended(&mut rx);
        assert!(matches!(outcome, Outcome::Signaled { signal, .. } if signal == libc::SIGSEGV));
        assert!(outcome.is_failure());
    }
}
