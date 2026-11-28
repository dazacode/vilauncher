//! Keyboard keys for the game's controller controls.
//!
//! VIctory's input layer reads a pad from the keyboard when no controller is connected:
//! a fixed table maps keys to pad buttons and sticks. Here the player can give each of
//! those controls another key. While the game window is in front, a low-level keyboard
//! hook sends the layer's own key for the control whose key was pressed. Off by default.
//! Only Windows has the hook; elsewhere the settings are kept and nothing is sent.

use std::collections::BTreeMap;

/// One control the layer reads from the keyboard, with the key the layer uses for it.
pub struct Control {
    pub id: &'static str,
    pub label: &'static str,
    /// Windows virtual-key code the layer reads (the same as the browser's keyCode).
    pub default_key: u16,
}

pub const CONTROLS: &[Control] = &[
    Control {
        id: "menu",
        label: "Menu",
        default_key: 0x1B,
    },
    Control {
        id: "view",
        label: "View",
        default_key: 0x09,
    },
    Control {
        id: "a",
        label: "A",
        default_key: 0x0D,
    },
    Control {
        id: "b",
        label: "B",
        default_key: 0x08,
    },
    Control {
        id: "x",
        label: "X",
        default_key: 0x58,
    },
    Control {
        id: "y",
        label: "Y",
        default_key: 0x59,
    },
    Control {
        id: "up",
        label: "D-pad up",
        default_key: 0x26,
    },
    Control {
        id: "down",
        label: "D-pad down",
        default_key: 0x28,
    },
    Control {
        id: "left",
        label: "D-pad left",
        default_key: 0x25,
    },
    Control {
        id: "right",
        label: "D-pad right",
        default_key: 0x27,
    },
    Control {
        id: "left_shoulder",
        label: "Left shoulder",
        default_key: 0x51,
    },
    Control {
        id: "right_shoulder",
        label: "Right shoulder",
        default_key: 0x45,
    },
    Control {
        id: "left_stick_click",
        label: "Left stick click",
        default_key: 0x46,
    },
    Control {
        id: "right_stick_click",
        label: "Right stick click",
        default_key: 0x48,
    },
    Control {
        id: "left_stick_up",
        label: "Left stick up",
        default_key: 0x57,
    },
    Control {
        id: "left_stick_left",
        label: "Left stick left",
        default_key: 0x41,
    },
    Control {
        id: "left_stick_down",
        label: "Left stick down",
        default_key: 0x53,
    },
    Control {
        id: "left_stick_right",
        label: "Left stick right",
        default_key: 0x44,
    },
    Control {
        id: "right_stick_up",
        label: "Right stick up",
        default_key: 0x49,
    },
    Control {
        id: "right_stick_left",
        label: "Right stick left",
        default_key: 0x4A,
    },
    Control {
        id: "right_stick_down",
        label: "Right stick down",
        default_key: 0x4B,
    },
    Control {
        id: "right_stick_right",
        label: "Right stick right",
        default_key: 0x4C,
    },
    Control {
        id: "left_trigger",
        label: "Left trigger",
        default_key: 0x5A,
    },
    Control {
        id: "right_trigger",
        label: "Right trigger",
        default_key: 0x43,
    },
];

pub fn control(id: &str) -> Option<&'static Control> {
    CONTROLS.iter().find(|c| c.id == id)
}

/// The key the player uses for a control: their own, else the layer's.
pub fn key_of(bindings: &BTreeMap<String, u16>, control: &Control) -> u16 {
    bindings
        .get(control.id)
        .copied()
        .unwrap_or(control.default_key)
}

/// Whether `key` may be given to control `id`: a real key, and not one another control uses.
pub fn check_key(bindings: &BTreeMap<String, u16>, id: &str, key: u16) -> Result<(), String> {
    // 0 and 0xFF are not keys; the rest stay with Windows or are modifiers the game needs.
    const RESERVED: &[u16] = &[
        0x03, 0x10, 0x11, 0x12, 0x14, 0x2C, 0x5B, 0x5C, 0x5D, 0x90, 0x91, 0xA0, 0xA1, 0xA2, 0xA3,
        0xA4, 0xA5,
    ];
    if key == 0 || key == 0xFF || RESERVED.contains(&key) {
        return Err("That key cannot be used for a controller control.".into());
    }
    for other in CONTROLS.iter().filter(|c| c.id != id) {
        if key_of(bindings, other) == key {
            return Err(format!("{} already uses that key.", other.label));
        }
    }
    Ok(())
}

/// What the game should see for a key the player pressed.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Unchanged: the key reaches the game as pressed.
    Pass,
    /// The layer's key for the control the player pressed.
    Send(u16),
    /// Nothing: the key is the layer's own key for a control the player moved elsewhere.
    Swallow,
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn translate(bindings: &BTreeMap<String, u16>, pressed: u16) -> Step {
    if let Some(control) = CONTROLS.iter().find(|c| key_of(bindings, c) == pressed) {
        return if control.default_key == pressed {
            Step::Pass
        } else {
            Step::Send(control.default_key)
        };
    }
    if CONTROLS.iter().any(|c| c.default_key == pressed) {
        Step::Swallow
    } else {
        Step::Pass
    }
}

#[cfg(windows)]
mod hook {
    use super::{translate, Step};
    use std::collections::{BTreeMap, HashMap};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, Once};
    use windows_sys::Win32::Foundation::{CloseHandle, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
        SetWindowsHookExW, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN,
        WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    /// The game's program name: every VIctory build starts `victory.exe`.
    const GAME_EXE: &str = "victory.exe";

    static ENABLED: AtomicBool = AtomicBool::new(false);
    static HOOK_STARTED: Once = Once::new();

    struct State {
        bindings: BTreeMap<String, u16>,
        /// Keys held down whose presses were changed, and what each sent. Kept across
        /// settings changes, so the release is always seen and nothing stays pressed.
        held: HashMap<u16, Step>,
    }

    static STATE: Mutex<Option<State>> = Mutex::new(None);

    fn lock() -> std::sync::MutexGuard<'static, Option<State>> {
        STATE.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn configure(on: bool, bindings: BTreeMap<String, u16>) {
        {
            let mut state = lock();
            let held = state.take().map(|old| old.held).unwrap_or_default();
            *state = Some(State { bindings, held });
        }
        ENABLED.store(on, Ordering::Relaxed);
        if on {
            HOOK_STARTED.call_once(|| {
                std::thread::spawn(|| unsafe {
                    let hook = SetWindowsHookExW(
                        WH_KEYBOARD_LL,
                        Some(keyboard_hook),
                        std::ptr::null_mut(),
                        0,
                    );
                    if hook.is_null() {
                        return;
                    }
                    let mut message: MSG = std::mem::zeroed();
                    // The hook runs on this thread, so it needs a message loop for as long as the launcher runs.
                    while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {}
                });
            });
        }
    }

    unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let info = &*(lparam as *const KBDLLHOOKSTRUCT);
            let message = wparam as u32;
            let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
            let up = message == WM_KEYUP || message == WM_SYSKEYUP;
            // Keys this hook sent are flagged injected: letting them through avoids a loop.
            if (down || up) && info.flags & LLKHF_INJECTED == 0 {
                let key = info.vkCode as u16;
                // A release is always handled, so a key pressed in the game cannot stay down
                // when the focus moves away or the setting changes.
                let consumed = if up {
                    release(key)
                } else {
                    ENABLED.load(Ordering::Relaxed) && game_in_front() && press(key)
                };
                if consumed {
                    return 1;
                }
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }

    /// A key went down in the game. Returns true when the key is consumed.
    fn press(key: u16) -> bool {
        let mut guard = lock();
        let Some(state) = guard.as_mut() else {
            return false;
        };
        if state.held.contains_key(&key) {
            return true;
        }
        match translate(&state.bindings, key) {
            Step::Pass => false,
            Step::Send(sent) => {
                state.held.insert(key, Step::Send(sent));
                drop(guard);
                // SAFETY: SendInput with one well-formed input.
                unsafe { send(sent, true) };
                true
            }
            Step::Swallow => {
                state.held.insert(key, Step::Swallow);
                true
            }
        }
    }

    /// A key went up. Releases what its press sent. Returns true when the key is consumed.
    fn release(key: u16) -> bool {
        let mut guard = lock();
        let Some(state) = guard.as_mut() else {
            return false;
        };
        match state.held.remove(&key) {
            None => false,
            Some(Step::Send(sent)) => {
                drop(guard);
                // SAFETY: as above.
                unsafe { send(sent, false) };
                true
            }
            Some(_) => true,
        }
    }

    /// Whether the window in front belongs to the game.
    unsafe fn game_in_front() -> bool {
        let window = GetForegroundWindow();
        if window.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(window, &mut pid);
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return false;
        }
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        let named = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) != 0;
        CloseHandle(process);
        named && {
            let path = String::from_utf16_lossy(&buffer[..length as usize]);
            std::path::Path::new(&path)
                .file_name()
                .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(GAME_EXE))
        }
    }

    unsafe fn send(key: u16, down: bool) {
        // Arrows, paging, insert and delete are extended keys.
        let extended = matches!(key, 0x21..=0x28 | 0x2D | 0x2E);
        let mut flags = 0;
        if !down {
            flags |= KEYEVENTF_KEYUP;
        }
        if extended {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key,
                    wScan: MapVirtualKeyW(u32::from(key), MAPVK_VK_TO_VSC) as u16,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Applies the player's settings: whether the keys are on, and their key choices.
pub fn configure(on: bool, bindings: &BTreeMap<String, u16>) {
    #[cfg(windows)]
    hook::configure(on, bindings.clone());
    #[cfg(not(windows))]
    let _ = (on, bindings);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bind(pairs: &[(&str, u16)]) -> BTreeMap<String, u16> {
        pairs
            .iter()
            .map(|(id, key)| (id.to_string(), *key))
            .collect()
    }

    #[test]
    fn with_no_changes_every_key_passes() {
        let none = BTreeMap::new();
        assert_eq!(translate(&none, 0x0D), Step::Pass);
        assert_eq!(translate(&none, 0x46), Step::Pass);
        assert_eq!(translate(&none, 0x20), Step::Pass);
    }

    #[test]
    fn a_moved_control_sends_the_layers_key_and_its_old_key_is_swallowed() {
        // A (Enter in the layer) moved to F: F sends Enter, Enter itself is swallowed,
        // and other keys are untouched.
        let moved = bind(&[("a", 0x46)]);
        assert_eq!(translate(&moved, 0x46), Step::Send(0x0D));
        assert_eq!(translate(&moved, 0x0D), Step::Swallow);
        assert_eq!(translate(&moved, 0x58), Step::Pass);
    }

    #[test]
    fn a_key_already_in_use_is_refused_and_a_freed_key_is_allowed() {
        let none = BTreeMap::new();
        // X is the layer's X key for control x: another control cannot take it.
        assert!(check_key(&none, "a", 0x58).is_err());
        // Once X moves to K, X is free for another control.
        let moved = bind(&[("x", 0x4B)]);
        assert!(check_key(&moved, "a", 0x58).is_ok());
        // Keys the layer never reads for a control are not refused for that reason.
        assert!(check_key(&none, "a", 0x20).is_ok());
        assert!(check_key(&none, "a", 0).is_err());
        // Modifiers, Windows and lock keys stay with the system.
        assert!(check_key(&none, "a", 0x12).is_err());
        assert!(check_key(&none, "a", 0x5B).is_err());
    }

    #[test]
    fn every_control_has_a_distinct_default_key() {
        for (i, a) in CONTROLS.iter().enumerate() {
            for b in &CONTROLS[i + 1..] {
                assert_ne!(a.default_key, b.default_key, "{} and {}", a.id, b.id);
            }
        }
    }
}
