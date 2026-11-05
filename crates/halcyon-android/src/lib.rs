//! Halcyon: the Android entry point of Horizon.
//!
//! The Kotlin app starts a GameActivity, which loads this library and calls `android_main`.

use winit::platform::android::activity::AndroidApp;

#[global_allocator]
static ALLOCATOR: horizon_threads::Allocator = horizon_threads::ALLOCATOR;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    horizon_client::eventloop::set_android_app(app.clone());
    // The game keeps its files under the app's own folder.
    if let Some(dir) = app.internal_data_path() {
        let _ = std::env::set_current_dir(&dir);
        // The host app writes the command line (one argument per line) before it starts the game.
        if let Ok(text) = std::fs::read_to_string(dir.join("horizon-args.txt")) {
            let args: Vec<String> = text
                .lines()
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect();
            horizon_client::eventloop::set_args(args);
        }
    }
    horizon_client::entry::main();
}
