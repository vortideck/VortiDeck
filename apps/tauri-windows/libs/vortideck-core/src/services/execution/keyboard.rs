/// Keyboard simulation utilities using Enigo
use enigo::{Direction, Enigo, Key, Keyboard};

/// Maps a string representation of a key to the corresponding Enigo Key enum variant
pub fn map_string_to_key(key: &str) -> Option<Key> {
    match key {
        // Modifiers
        "LShift" | "RShift" | "Shift" => Some(Key::Shift),
        "LControl" | "RControl" | "Control" => Some(Key::Control),
        "LAlt" | "RAlt" | "Alt" => Some(Key::Alt),
        "Command" | "RCommand" | "LMeta" | "RMeta" | "Meta" => Some(Key::Meta),

        // Controls
        "Escape" => Some(Key::Escape),
        "Space" => Some(Key::Space),
        "Enter" | "Return" => Some(Key::Return),
        "Tab" => Some(Key::Tab),
        "Backspace" => Some(Key::Backspace),
        "CapsLock" => Some(Key::CapsLock),
        "NumLock" => Some(Key::Numlock),

        // Navigation
        "Up" => Some(Key::UpArrow),
        "Down" => Some(Key::DownArrow),
        "Left" => Some(Key::LeftArrow),
        "Right" => Some(Key::RightArrow),
        "PageUp" => Some(Key::PageUp),
        "PageDown" => Some(Key::PageDown),
        "Home" => Some(Key::Home),
        "End" => Some(Key::End),
        "Insert" => Some(Key::Insert),
        "Delete" => Some(Key::Delete),

        // Function keys
        "F1" => Some(Key::F1),
        "F2" => Some(Key::F2),
        "F3" => Some(Key::F3),
        "F4" => Some(Key::F4),
        "F5" => Some(Key::F5),
        "F6" => Some(Key::F6),
        "F7" => Some(Key::F7),
        "F8" => Some(Key::F8),
        "F9" => Some(Key::F9),
        "F10" => Some(Key::F10),
        "F11" => Some(Key::F11),
        "F12" => Some(Key::F12),
        "F13" => Some(Key::F13),
        "F14" => Some(Key::F14),
        "F15" => Some(Key::F15),
        "F16" => Some(Key::F16),
        "F17" => Some(Key::F17),
        "F18" => Some(Key::F18),
        "F19" => Some(Key::F19),
        "F20" => Some(Key::F20),

        // Numbers from device_query (e.g., Key0 to Key9)
        "Key0" => Some(Key::Unicode('0')),
        "Key1" => Some(Key::Unicode('1')),
        "Key2" => Some(Key::Unicode('2')),
        "Key3" => Some(Key::Unicode('3')),
        "Key4" => Some(Key::Unicode('4')),
        "Key5" => Some(Key::Unicode('5')),
        "Key6" => Some(Key::Unicode('6')),
        "Key7" => Some(Key::Unicode('7')),
        "Key8" => Some(Key::Unicode('8')),
        "Key9" => Some(Key::Unicode('9')),

        // Numpad keys
        "Numpad0" => Some(Key::Unicode('0')),
        "Numpad1" => Some(Key::Unicode('1')),
        "Numpad2" => Some(Key::Unicode('2')),
        "Numpad3" => Some(Key::Unicode('3')),
        "Numpad4" => Some(Key::Unicode('4')),
        "Numpad5" => Some(Key::Unicode('5')),
        "Numpad6" => Some(Key::Unicode('6')),
        "Numpad7" => Some(Key::Unicode('7')),
        "Numpad8" => Some(Key::Unicode('8')),
        "Numpad9" => Some(Key::Unicode('9')),
        "NumpadSubtract" => Some(Key::Unicode('-')),
        "NumpadAdd" => Some(Key::Unicode('+')),
        "NumpadDivide" => Some(Key::Unicode('/')),
        "NumpadMultiply" => Some(Key::Unicode('*')),
        "NumpadEquals" => Some(Key::Unicode('=')),
        "NumpadEnter" => Some(Key::Return),
        "NumpadDecimal" => Some(Key::Unicode('.')),

        // Symbols
        "Semicolon" => Some(Key::Unicode(';')),
        "Equal" => Some(Key::Unicode('=')),
        "Comma" => Some(Key::Unicode(',')),
        "Minus" => Some(Key::Unicode('-')),
        "Dot" | "Period" => Some(Key::Unicode('.')),
        "Slash" => Some(Key::Unicode('/')),
        "Grave" => Some(Key::Unicode('`')),
        "LeftBracket" => Some(Key::Unicode('[')),
        "BackSlash" => Some(Key::Unicode('\\')),
        "RightBracket" => Some(Key::Unicode(']')),
        "Apostrophe" => Some(Key::Unicode('\'')),

        _ => {
            // Default mapping for single character letters or other unknown strings
            if key.len() == 1 {
                if let Some(c) = key.chars().next() {
                    let lower_c = c.to_lowercase().next().unwrap();
                    return Some(Key::Unicode(lower_c));
                }
            } else if key.starts_with("Key") && key.len() == 4 {
                // Handle keynames like "KeyA", "KeyB" if they happen
                if let Some(c) = key.chars().nth(3) {
                    let lower_c = c.to_lowercase().next().unwrap();
                    return Some(Key::Unicode(lower_c));
                }
            }
            None
        }
    }
}

/// Simulate pressing a key down
pub fn simulate_key_down(enigo: &mut Enigo, key: &str) -> Result<(), String> {
    log::debug!("Simulating key down: {}", key);

    if let Some(mapped_key) = map_string_to_key(key) {
        enigo
            .key(mapped_key, Direction::Press)
            .map_err(|e| format!("Failed to press key {}: {:?}", key, e))?;
    } else {
        // Fallback to typing the character if it's a single character
        if let Some(c) = key.chars().next() {
            let lower_c = c.to_lowercase().next().unwrap();
            enigo
                .key(Key::Unicode(lower_c), Direction::Press)
                .map_err(|e| format!("Failed to press key '{}': {:?}", c, e))?;
        } else {
            return Err(format!("Unsupported key representation: {}", key));
        }
    }

    Ok(())
}

/// Simulate releasing a key
pub fn simulate_key_up(enigo: &mut Enigo, key: &str) -> Result<(), String> {
    log::debug!("Simulating key up: {}", key);

    if let Some(mapped_key) = map_string_to_key(key) {
        enigo
            .key(mapped_key, Direction::Release)
            .map_err(|e| format!("Failed to release key {}: {:?}", key, e))?;
    } else {
        // Fallback to releasing the character if it's a single character
        if let Some(c) = key.chars().next() {
            let lower_c = c.to_lowercase().next().unwrap();
            enigo
                .key(Key::Unicode(lower_c), Direction::Release)
                .map_err(|e| format!("Failed to release key '{}': {:?}", c, e))?;
        } else {
            return Err(format!("Unsupported key representation: {}", key));
        }
    }

    Ok(())
}
