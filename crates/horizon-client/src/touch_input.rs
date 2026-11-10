//! Halcyon (Android): what the launcher's on-screen controls hand to the game.
//!
//! The overlay (Kotlin) sends plain numbers through JNI; they become the game's own control lines
//! (`key w down`, `mouse left up`, `hotbar 3`) and run where the control socket's lines run. The list of
//! key names is shared with the launcher: its order must stay as it is, new keys only go at the end.

use std::sync::{Mutex, atomic::AtomicI32};

/// The keys a button can press, by the number the launcher sends.
pub const KEYS: &[&str] = &[
    "w", "a", "s", "d", "space", "shift", "ctrl", "e", "q", "t", //  0..9
    "f", "esc", "tab", "f1", "f2", "f3", "f4", "f5", "f11", "slash", // 10..19
    "enter", "up", "down", "left", "right", "r", "g", "c", "v", "b", // 20..29
    "z", "x", "1", "2", "3", "4", "5", "6", "7", "8", // 30..39
    "9", "pageup", "pagedown", "backspace", // 40..43
];

static QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Where the game is: 0 the title menu or loading, 1 playing, 2 a screen (inventory, pause, ...).
pub static STATE: AtomicI32 = AtomicI32::new(0);

/// One input from the launcher: `kind` 0 key down, 1 key up (`code` is the index in [`KEYS`]), 2 mouse
/// button down, 3 mouse button up (`code` 0 left, 1 right, 2 middle), 4 turn by `x`, `y` degrees,
/// 5 hotbar slot `code` (1..=9).
pub fn push(kind: i32, code: i32, x: f32, y: f32) {
    let index = usize::try_from(code).unwrap_or(usize::MAX);
    let line = match kind {
        0 => KEYS.get(index).map(|name| format!("key {name} down")),
        1 => KEYS.get(index).map(|name| format!("key {name} up")),
        2 | 3 => ["left", "right", "middle"].get(index).map(|button| {
            let action = if kind == 2 { "down" } else { "up" };
            format!("mouse {button} {action}")
        }),
        4 => Some(format!("turn {x} {y}")),
        5 => (1..=9).contains(&code).then(|| format!("hotbar {code}")),
        _ => None,
    };
    if let Some(line) = line
        && let Ok(mut queue) = QUEUE.lock()
        && queue.len() < 512
    {
        queue.push(line);
    }
}

/// The lines waiting, oldest first.
pub fn drain() -> Vec<String> {
    QUEUE
        .lock()
        .map(|mut queue| std::mem::take(&mut *queue))
        .unwrap_or_default()
}
