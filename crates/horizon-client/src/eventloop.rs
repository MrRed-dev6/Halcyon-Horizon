//! The winit event loop: plain on desktop, bound to the Android app on Android (Halcyon).

use winit::error::EventLoopError;
use winit::event_loop::EventLoop;

#[cfg(target_os = "android")]
static ANDROID_APP: std::sync::OnceLock<winit::platform::android::activity::AndroidApp> =
    std::sync::OnceLock::new();

/// Android: the app handed to `android_main`, kept for the event loop.
#[cfg(target_os = "android")]
pub fn set_android_app(app: winit::platform::android::activity::AndroidApp) {
    let _ = ANDROID_APP.set(app);
}

/// The event loop the game runs on.
pub fn new() -> Result<EventLoop<()>, EventLoopError> {
    #[cfg(target_os = "android")]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        let app = ANDROID_APP
            .get()
            .expect("Android: set_android_app must run before the event loop")
            .clone();
        EventLoop::builder().with_android_app(app).build()
    }
    #[cfg(not(target_os = "android"))]
    {
        EventLoop::new()
    }
}

static ARGS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// Halcyon: the command line handed over by the host app (Android has no process arguments).
pub fn set_args(args: Vec<String>) {
    let _ = ARGS.set(args);
}

/// The command line without the program name: the host app's, or the process's own.
pub fn args() -> Vec<String> {
    ARGS.get()
        .cloned()
        .unwrap_or_else(|| std::env::args().skip(1).collect())
}
