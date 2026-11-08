//! Halcyon: the Android entry point of Horizon.
//!
//! The Kotlin app starts a GameActivity, which loads this library and calls `android_main`.
//! Everything that happens before the game's own logging starts goes to `horizon-startup.log`
//! in the app's files folder, which the launcher shows on its "Game log" screen.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use winit::platform::android::activity::AndroidApp;

#[global_allocator]
static ALLOCATOR: horizon_threads::Allocator = horizon_threads::ALLOCATOR;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

fn note(dir: &Path, text: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("horizon-startup.log"))
    {
        let _ = writeln!(file, "{text}");
    }
}

extern "C" fn at_exit() {
    if let Some(dir) = LOG_DIR.get() {
        note(dir, "process exit (atexit)");
    }
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    if let Some(dir) = app.internal_data_path() {
        // A fresh log for every start.
        let _ = std::fs::write(dir.join("horizon-startup.log"), "");
        let _ = LOG_DIR.set(dir.clone());
        let hook_dir = dir.clone();
        std::panic::set_hook(Box::new(move |info| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            note(&hook_dir, &format!("PANIC: {info}\n{backtrace}"));
        }));
        // SAFETY: registers a plain function that only appends a line to a file.
        unsafe {
            libc::atexit(at_exit);
        }
        note(&dir, "android_main: started");
    }

    horizon_client::eventloop::set_android_app(app.clone());
    if let Some(dir) = app.internal_data_path() {
        let _ = std::env::set_current_dir(&dir);
        // The host app writes the command line (one argument per line) before it starts the game.
        if let Ok(text) = std::fs::read_to_string(dir.join("horizon-args.txt")) {
            let args: Vec<String> = text
                .lines()
                .filter(|line| !line.is_empty())
                // The session file's path is shown, never its content.
                .map(str::to_owned)
                .collect();
            note(&dir, &format!("arguments: {args:?}"));
            horizon_client::eventloop::set_args(args);
        } else {
            note(&dir, "no horizon-args.txt");
        }
        note(&dir, "entering the game");
    }
    horizon_client::entry::main();
    if let Some(dir) = LOG_DIR.get() {
        note(dir, "the game returned");
    }
}
