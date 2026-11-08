//! Halcyon (Android): how fast the game draws, in the log every few seconds, to find where a stutter comes from.

use std::sync::Mutex;
use std::time::Instant;

struct Stats {
    since: Instant,
    last: Instant,
    frames: u32,
    worst_ms: f32,
}

static STATS: Mutex<Option<Stats>> = Mutex::new(None);

/// The first window events other than redraws are logged ("Horizon event: ..."): do touches arrive at all?
pub fn note_event(event: &winit::event::WindowEvent) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static LOGGED: AtomicU32 = AtomicU32::new(0);
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 80 {
        let text = format!("{event:?}");
        log::info!("Horizon event: {}", text.chars().take(110).collect::<String>());
    }
}

/// Call once per redraw. Logs "Horizon frames: ..." every five seconds.
pub fn tick() {
    let now = Instant::now();
    let mut slot = STATS.lock().unwrap_or_else(|e| e.into_inner());
    let stats = slot.get_or_insert(Stats {
        since: now,
        last: now,
        frames: 0,
        worst_ms: 0.0,
    });
    let dt = now.duration_since(stats.last).as_secs_f32() * 1000.0;
    stats.last = now;
    stats.frames += 1;
    stats.worst_ms = stats.worst_ms.max(dt);
    let span = now.duration_since(stats.since).as_secs_f32();
    if span >= 5.0 {
        log::info!(
            "Horizon frames: {} in {:.1} s = {:.1} fps, longest frame {:.0} ms",
            stats.frames,
            span,
            stats.frames as f32 / span,
            stats.worst_ms
        );
        *stats = Stats {
            since: now,
            last: now,
            frames: 0,
            worst_ms: 0.0,
        };
    }
}
