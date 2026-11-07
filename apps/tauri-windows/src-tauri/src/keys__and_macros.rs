use std::collections::HashMap;
use enigo::{
    Button, Coordinate,
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};
use serde_json;
use tokio;

#[derive(Debug, Clone)]
struct Command {
    button_id: String,
    action: String,
    data: Option<String>,
}

fn create_key_map() -> HashMap<String, Key> {
    let mut key_map = HashMap::new();
    key_map.insert("Backspace".to_string(), Key::Backspace);
    key_map.insert("Enter".to_string(), Key::Return);
    key_map.insert("Space".to_string(), Key::Space);
    key_map.insert("A".to_string(), Key::Layout('a'));
    key_map.insert("B".to_string(), Key::Layout('b'));
    // Add more keys as needed
    key_map
}

async fn execute_command(command: &Command, key_map: &HashMap<String, Key>) {
    match command.action.as_str() {
        "keypress" => {
            if let Some(key_name) = &command.data {
                execute_keypress(key_name, key_map).await;
            } else {
                println!("Key data missing for command");
            }
        }
        "macro" => {
            if let Some(macro_data) = &command.data {
                if let Ok(commands) = serde_json::from_str::<Vec<String>>(macro_data) {
                    for cmd in commands {
                        let macro_command = Command {
                            button_id: "macro".to_string(),
                            action: "keypress".to_string(),
                            data: Some(cmd),
                        };
                        execute_command(&macro_command, key_map).await;
                    }
                }
            }
        }
        _ => println!("Unknown action: {}", command.action),
    }
}

async fn execute_keypress(key_name: &str, key_map: &HashMap<String, Key>) {
    let enigo = Enigo::new(&Settings::default());
    if let Ok(mut enigo) = enigo {
        if let Some(&key) = key_map.get(key_name) {
            println!("Executing keypress for: {}", key_name);
            enigo.key(key, enigo::Direction::Click);
        } else {
            println!("Unknown key: {}", key_name);
        }
    } else {
        println!("Failed to initialize Enigo");
    }
}

#[tokio::main]
async fn main() {
    let key_map = create_key_map();

    let command = Command {
        button_id: "btn1".to_string(),
        action: "keypress".to_string(),
        data: Some("Backspace".to_string()),
    };

    execute_command(&command, &key_map).await;
}
