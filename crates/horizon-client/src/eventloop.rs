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
