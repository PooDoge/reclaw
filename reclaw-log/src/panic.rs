//! A panic goes into the log with where it happened, which thread, and the stack, before the usual message on stderr. Without this a
//! panic in a launcher started from a menu entry leaves nothing behind but a window that closed.
use std::sync::Once;

static INSTALL: Once = Once::new();

/// The text a panic carried. `panic!("x")` carries a `&str`, `panic!("{x}")` a `String`.
fn message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "(the panic carried something that is not text)".to_string())
}

/// Install the hook once; the hook that was there before still runs afterwards.
pub(crate) fn install() {
    INSTALL.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let thread = std::thread::current();
            let location = info.location().map_or_else(|| "an unknown place".to_string(), |l| format!("{}:{}", l.file(), l.line()));
            let backtrace = std::backtrace::Backtrace::force_capture();
            tracing::error!(
                target: "reclaw::panic",
                thread = thread.name().unwrap_or("(unnamed)"),
                %location,
                "panic: {}\n{backtrace}",
                message(info.payload())
            );
            previous(info);
        }));
    });
}
