//! Halcyon (Android): a finger is the mouse in the menus, and no pointer is drawn.
//!
//! A tap clicks where it lands. A drag that goes sideways holds the left button from where it began, so a
//! slider follows the finger. A drag that goes up or down scrolls the list under it, as far as the finger
//! moved. Only the first finger counts. [`translate`] turns winit's `Touch` events into the mouse events the
//! screens already understand and hands every other event back as it came.

use std::sync::{
    Mutex,
    atomic::{AtomicU32, Ordering},
};

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase, WindowEvent};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// The finger has not moved far enough to be anything but a tap yet.
    Undecided,
    /// Sideways: the left button is held.
    Drag,
    /// Up or down: the list under the finger scrolls.
    Scroll,
}

struct Finger {
    id: u64,
    start: PhysicalPosition<f64>,
    last: PhysicalPosition<f64>,
    mode: Mode,
}

static FINGER: Mutex<Option<Finger>> = Mutex::new(None);

/// The first touch events are logged ("Horizon touch: ..."), to see what a finger turned into.
static LOGGED: AtomicU32 = AtomicU32::new(0);
const LOG_LIMIT: u32 = 60;

/// A finger that moves less than this (in device-independent pixels) still taps.
const SLOP_DP: f64 = 10.0;

/// One wheel notch is about this many GUI pixels of a list (half an entry), so a list follows the finger.
const NOTCH_GUI_PIXELS: f64 = 14.0;

/// Physical pixels per GUI pixel: the chosen GUI scale, or the largest that fits like vanilla's automatic one.
pub fn gui_pixel(width: u32, height: u32) -> f64 {
    let chosen = horizon_client::hud::GUI_SCALE.load(Ordering::Relaxed);
    if chosen > 0 {
        return f64::from(chosen);
    }
    ((f64::from(width) / 320.0).floor().min((f64::from(height) / 240.0).floor())).max(1.0)
}

/// The mouse events for a touch event, or the event itself when it is not one.
pub fn translate(event: WindowEvent, scale_factor: f64, gui_pixel: f64) -> Vec<WindowEvent> {
    let touch = match event {
        WindowEvent::Touch(touch) => touch,
        other => return vec![other],
    };
    let Touch {
        device_id,
        phase,
        location,
        id,
        ..
    } = touch;
    let moved = |position: PhysicalPosition<f64>| WindowEvent::CursorMoved { device_id, position };
    let button = |state: ElementState| WindowEvent::MouseInput {
        device_id,
        state,
        button: MouseButton::Left,
    };
    let mut slot = FINGER.lock().unwrap_or_else(|e| e.into_inner());
    let mut out = Vec::new();
    let log_it = LOGGED.fetch_add(1, Ordering::Relaxed) < LOG_LIMIT;
    match phase {
        TouchPhase::Started => {
            if slot.is_none() {
                *slot = Some(Finger {
                    id,
                    start: location,
                    last: location,
                    mode: Mode::Undecided,
                });
                out.push(moved(location));
            }
        }
        TouchPhase::Moved => {
            if let Some(finger) = slot.as_mut().filter(|f| f.id == id) {
                if finger.mode == Mode::Undecided {
                    let (dx, dy) = (location.x - finger.start.x, location.y - finger.start.y);
                    if dx.hypot(dy) >= SLOP_DP * scale_factor {
                        if dx.abs() > dy.abs() {
                            finger.mode = Mode::Drag;
                            out.push(moved(finger.start));
                            out.push(button(ElementState::Pressed));
                        } else {
                            finger.mode = Mode::Scroll;
                        }
                    }
                }
                match finger.mode {
                    Mode::Drag => out.push(moved(location)),
                    Mode::Scroll => {
                        // Content follows the finger: moving down shows what is above, like a wheel turned up.
                        let notches = (location.y - finger.last.y) / (NOTCH_GUI_PIXELS * gui_pixel.max(1.0));
                        out.push(moved(location));
                        out.push(WindowEvent::MouseWheel {
                            device_id,
                            delta: MouseScrollDelta::LineDelta(0.0, notches as f32),
                            phase: TouchPhase::Moved,
                        });
                    }
                    Mode::Undecided => {}
                }
                finger.last = location;
            }
        }
        TouchPhase::Ended | TouchPhase::Cancelled => {
            if let Some(finger) = slot.take_if(|f| f.id == id) {
                match finger.mode {
                    Mode::Undecided if phase == TouchPhase::Ended => {
                        out.push(moved(location));
                        out.push(button(ElementState::Pressed));
                        out.push(button(ElementState::Released));
                    }
                    Mode::Drag => {
                        out.push(moved(location));
                        out.push(button(ElementState::Released));
                    }
                    _ => {}
                }
            }
        }
    }
    if log_it {
        log::info!(
            "Horizon touch: {phase:?} at ({:.0}, {:.0}) finger {id}, mode {}, {} mouse events",
            location.x,
            location.y,
            match slot.as_ref().map(|f| f.mode) {
                Some(Mode::Drag) => "drag",
                Some(Mode::Scroll) => "scroll",
                Some(Mode::Undecided) => "undecided",
                None => "none",
            },
            out.len()
        );
    }
    out
}

/// What a finger does in the world (not in a menu): the first one that lands outside the hotbar turns the
/// view, a tap on a hotbar slot picks it. The on-screen buttons are the launcher's, they never get here.
pub enum World {
    Nothing,
    /// A hotbar slot, 0 to 8.
    Hotbar(usize),
    /// The view turns by this much, in pixels, as a mouse's motion would.
    Look(f64, f64),
}

static LOOK: Mutex<Option<(u64, PhysicalPosition<f64>)>> = Mutex::new(None);

/// The hotbar slot under a finger, if it is on the hotbar (a little taller than it is drawn).
fn hotbar_slot(position: PhysicalPosition<f64>, width: u32, height: u32) -> Option<usize> {
    let pixel = gui_pixel(width, height);
    let (x, y) = (position.x / pixel, position.y / pixel);
    let (w, h) = (f64::from(width) / pixel, f64::from(height) / pixel);
    let left = w / 2.0 - 91.0;
    if y < h - 30.0 || x < left || x >= left + 182.0 {
        return None;
    }
    Some((((x - left - 3.0) / 20.0).floor().max(0.0) as usize).min(8))
}

pub fn world(touch: &Touch, width: u32, height: u32) -> World {
    let mut look = LOOK.lock().unwrap_or_else(|e| e.into_inner());
    match touch.phase {
        TouchPhase::Started => {
            if let Some(slot) = hotbar_slot(touch.location, width, height) {
                return World::Hotbar(slot);
            }
            if look.is_none() {
                *look = Some((touch.id, touch.location));
            }
            World::Nothing
        }
        TouchPhase::Moved => match look.as_mut() {
            Some((id, last)) if *id == touch.id => {
                let delta = World::Look(touch.location.x - last.x, touch.location.y - last.y);
                *last = touch.location;
                delta
            }
            _ => World::Nothing,
        },
        TouchPhase::Ended | TouchPhase::Cancelled => {
            if look.as_ref().is_some_and(|(id, _)| *id == touch.id) {
                *look = None;
            }
            World::Nothing
        }
    }
}
