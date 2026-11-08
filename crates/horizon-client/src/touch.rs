//! Halcyon (Android): a finger is the mouse in the menus, and no pointer is drawn.
//!
//! A tap clicks where it lands. A drag that goes sideways holds the left button from where it began, so a
//! slider follows the finger. A drag that goes up or down scrolls the list under it, as far as the finger
//! moved. Only the first finger counts. [`translate`] turns winit's `Touch` events into the mouse events the
//! screens already understand and hands every other event back as it came.

use std::sync::{Mutex, atomic::Ordering};

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
    out
}
