//! Chat eligibility and identity; effect execution remains in the invocation service.

use super::models::{ChatMessage, CommandInputSource, SessionCommand};
use crate::db::models::Action;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

const PLATFORMS: [&str; 5] = ["twitch", "youtube", "kick", "tiktok", "discord"];

pub fn normalize_commands(session_id: &str, commands: &mut [SessionCommand]) {
    for (index, command) in commands.iter_mut().enumerate() {
        if command.id.is_empty() {
            let identity = serde_json::json!([session_id, index, command.trigger]).to_string();
            command.id = format!("command-{:x}", Sha256::digest(identity.as_bytes()));
        }
        if command.permission_version == 0 {
            let roles = command.allowed_roles.get_or_insert_with(HashMap::new);
            for platform in PLATFORMS {
                let selected = roles.entry(platform.to_string()).or_default();
                if selected.is_empty() {
                    selected.push("everyone".into());
                }
            }
            command.permission_version = 1;
        }
    }
}

pub fn validate_commands(commands: &[SessionCommand]) -> Result<(), String> {
    let mut identities = HashSet::new();
    let mut triggers = HashSet::new();
    for command in commands {
        if command.permission_version != 1 || command.id.is_empty() {
            return Err("Unsupported command policy or missing command identity".into());
        }
        if !identities.insert(&command.id) {
            return Err("Commands must have distinct identities".into());
        }
        if !command.enabled {
            continue;
        }
        if command.trigger.is_empty()
            || command.trigger.chars().any(char::is_whitespace)
            || command.trigger.len() > 128
        {
            return Err(
                "Enabled commands require a nonempty trigger without whitespace (max 128 bytes)"
                    .into(),
            );
        }
        if !triggers.insert(&command.trigger) {
            return Err(format!(
                "Duplicate enabled command trigger '{}'",
                command.trigger
            ));
        }
        let action: Action = serde_json::from_str(&command.action)
            .map_err(|_| format!("Command '{}' has an invalid action", command.trigger))?;
        let valid = match action {
            Action::MacroId(id)
            | Action::Program(id)
            | Action::HttpRequestId(id)
            | Action::FocusWindowId(id) => id > 0,
            Action::SavedActionId(id) | Action::FlowId(id) | Action::App(id) => {
                !id.trim().is_empty() && !id.contains(['{', '}'])
            }
            Action::SystemControl(value) => !value.trim().is_empty(),
        };
        if !valid {
            return Err(format!(
                "Command '{}' has an invalid action target",
                command.trigger
            ));
        }
        if command.input_bindings.len() > 128 || command.input_bindings.iter().any(|(name, source)| {
            name.is_empty() || name.len() > 128 || matches!(source, CommandInputSource::Literal { value } if value.is_object() || value.is_array() || value.to_string().len() > 65_536)
        }) {
            return Err("Invalid chat command input bindings".into());
        }
        if let Some(roles) = &command.allowed_roles {
            for (platform, values) in roles {
                if !PLATFORMS.contains(&platform.as_str())
                    || values
                        .iter()
                        .any(|value| !platform_roles(platform).contains(&value.as_str()))
                {
                    return Err(format!(
                        "Command '{}' has unsupported platform roles",
                        command.trigger
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn command_inputs(
    command: &SessionCommand,
    message: &ChatMessage,
    args: &str,
    platform: &str,
) -> Result<serde_json::Value, String> {
    let mut inputs = serde_json::Map::new();
    for (name, source) in &command.input_bindings {
        let value = match source {
            CommandInputSource::Args => serde_json::json!(args),
            CommandInputSource::ArgsNumber => {
                if args.is_empty() {
                    continue;
                }
                serde_json::Value::Number(args_number(args)?)
            }
            CommandInputSource::ArgsBoolean => {
                if args.is_empty() {
                    continue;
                }
                serde_json::Value::Bool(
                    args.parse::<bool>()
                        .map_err(|_| "Command arguments must be true or false")?,
                )
            }
            CommandInputSource::Message => serde_json::json!(message.message),
            CommandInputSource::Username => serde_json::json!(message.user_name),
            CommandInputSource::DisplayName => serde_json::json!(message.user_display_name),
            CommandInputSource::Platform => serde_json::json!(platform),
            CommandInputSource::Literal { value } => value.clone(),
        };
        inputs.insert(name.clone(), value);
    }
    Ok(serde_json::Value::Object(inputs))
}

/// Whole numbers bind as JSON integers ("600" -> 600, not 600.0) so Integer
/// inputs accept them; fractional values stay floats.
fn args_number(args: &str) -> Result<serde_json::Number, String> {
    if let Ok(integer) = args.parse::<i64>() {
        return Ok(integer.into());
    }
    let value = args
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or("Command arguments must be a finite number")?;
    // i64::MAX as f64 rounds up to 2^63, so the upper bound is exclusive.
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value < i64::MAX as f64 {
        return Ok((value as i64).into());
    }
    serde_json::Number::from_f64(value)
        .ok_or_else(|| "Command arguments must be a finite number".into())
}

pub fn command_args<'a>(trigger: &str, message: &'a str) -> Option<&'a str> {
    if trigger.is_empty() || trigger.chars().any(char::is_whitespace) {
        return None;
    }
    let suffix = message.strip_prefix(trigger)?;
    if suffix.is_empty() || suffix.chars().next().is_some_and(char::is_whitespace) {
        Some(suffix.trim())
    } else {
        None
    }
}

pub fn check_command_permission(
    command: &SessionCommand,
    message: &ChatMessage,
    platform: &str,
) -> bool {
    // Lists match the stable provider user id; login names are still accepted
    // so existing name-based lists keep working. An id entry cannot be evaded
    // by renaming, so blacklists should prefer ids.
    let user_id = message.user_id.trim().to_lowercase();
    let username = message.user_name.trim().to_lowercase();
    let matches_user = |user: &String| {
        let user = user.trim().to_lowercase();
        !user.is_empty() && (user == user_id || user == username)
    };
    if command
        .user_blacklist
        .as_ref()
        .is_some_and(|users| users.iter().any(matches_user))
    {
        return false;
    }
    // Retain the explicit whitelist override: only listed users, irrespective of roles.
    if let Some(users) = command
        .user_whitelist
        .as_ref()
        .filter(|users| !users.is_empty())
    {
        return users.iter().any(matches_user);
    }
    let selected = command
        .allowed_roles
        .as_ref()
        .and_then(|roles| roles.get(platform));
    // Compatibility for callers that have not loaded through SessionManager yet.
    if command.permission_version == 0 && selected.is_none_or(Vec::is_empty) {
        return true;
    }
    let Some(selected) = selected else {
        return false;
    };
    let roles = get_user_roles(message, platform);
    selected.iter().any(|role| roles.contains(role))
}

fn platform_roles(platform: &str) -> &'static [&'static str] {
    match platform {
        "twitch" => &["everyone", "subscriber", "vip", "moderator", "broadcaster"],
        "youtube" => &["everyone", "verified", "member", "moderator", "owner"],
        "kick" => &["everyone", "subscriber", "moderator", "broadcaster"],
        _ => &["everyone"],
    }
}

/// Provider IDs are mandatory for consequential chat commands. No content hash
/// fallback: two legitimate identical messages must remain different deliveries.
pub fn delivery_identity(
    platform: &str,
    account: u64,
    session: &str,
    command: &str,
    message: &str,
) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("Chat command requires a stable provider message ID".into());
    }
    let identity = serde_json::json!([platform, account, session, command, message]).to_string();
    Ok(format!(
        "chat-command-{:x}",
        Sha256::digest(identity.as_bytes())
    ))
}

/// Old history/backfill must not become new effects after invocation retention.
pub fn validate_delivery_time(
    timestamp: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), String> {
    let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|_| "Chat command requires a valid provider timestamp")?;
    let age = now.signed_duration_since(timestamp).num_seconds();
    if !(-120..=300).contains(&age) {
        return Err("Chat command delivery is expired or has a future timestamp".into());
    }
    Ok(())
}

/// Legacy substitution operates on string values, never on serialized JSON or
/// property names. Saved action IDs are fixed by native command validation.
pub fn substitute_action(
    action: &str,
    variables: &HashMap<String, String>,
) -> Result<Action, String> {
    fn replace(value: &mut serde_json::Value, variables: &HashMap<String, String>) {
        match value {
            serde_json::Value::String(text) => {
                // One pass avoids recursively interpreting placeholders from user input.
                let source = text.clone();
                let mut result = String::new();
                let mut remaining = source.as_str();
                while let Some(start) = remaining.find('{') {
                    result.push_str(&remaining[..start]);
                    let candidate = &remaining[start..];
                    if let Some(end) = candidate.find('}') {
                        let key = &candidate[1..end];
                        if let Some(replacement) = variables.get(key) {
                            result.push_str(replacement);
                        } else {
                            result.push_str(&candidate[..=end]);
                        }
                        remaining = &candidate[end + 1..];
                    } else {
                        remaining = candidate;
                        break;
                    }
                }
                result.push_str(remaining);
                *text = result;
            }
            serde_json::Value::Array(items) => {
                items.iter_mut().for_each(|item| replace(item, variables))
            }
            serde_json::Value::Object(fields) => fields
                .values_mut()
                .for_each(|item| replace(item, variables)),
            _ => {}
        }
    }
    let mut value = serde_json::from_str(action).map_err(|_| "Invalid command action JSON")?;
    replace(&mut value, variables);
    serde_json::from_value(value).map_err(|_| "Invalid command action".to_string())
}

/// Extract user roles from chat message based on platform
fn get_user_roles(msg: &ChatMessage, platform: &str) -> Vec<String> {
    let mut roles = vec![];

    // Add platform-specific roles based on badges/flags
    match platform {
        "twitch" => {
            // Check badges for broadcaster/moderator/subscriber/vip
            if !msg.badges.is_empty() {
                if msg.badges.iter().any(|b| b == "broadcaster") {
                    roles.push("broadcaster".to_string());
                }
                if msg.badges.iter().any(|b| b == "moderator") {
                    roles.push("moderator".to_string());
                }
                if msg.badges.iter().any(|b| b == "subscriber")
                    || msg.badges.iter().any(|b| b.contains("subscriber"))
                {
                    roles.push("subscriber".to_string());
                }
                if msg.badges.iter().any(|b| b == "vip") {
                    roles.push("vip".to_string());
                }
            }
            // Also check boolean flags
            if msg.is_moderator && !roles.contains(&"moderator".to_string()) {
                roles.push("moderator".to_string());
            }
            if msg.is_subscriber && !roles.contains(&"subscriber".to_string()) {
                roles.push("subscriber".to_string());
            }
            if msg.is_vip && !roles.contains(&"vip".to_string()) {
                roles.push("vip".to_string());
            }
        }
        "youtube" => {
            // YouTube roles from badges
            if !msg.badges.is_empty() {
                if msg.badges.iter().any(|b| b == "owner") {
                    roles.push("owner".to_string());
                }
                if msg.badges.iter().any(|b| b == "moderator") {
                    roles.push("moderator".to_string());
                }
                if msg.badges.iter().any(|b| b == "member") {
                    roles.push("member".to_string());
                }
                if msg.badges.iter().any(|b| b == "verified") {
                    roles.push("verified".to_string());
                }
            }
            // Boolean flags
            if msg.is_moderator && !roles.contains(&"moderator".to_string()) {
                roles.push("moderator".to_string());
            }
        }
        "kick" => {
            // Kick roles
            if !msg.badges.is_empty() {
                if msg.badges.iter().any(|b| b == "broadcaster") {
                    roles.push("broadcaster".to_string());
                }
                if msg.badges.iter().any(|b| b == "moderator") {
                    roles.push("moderator".to_string());
                }
                if msg.badges.iter().any(|b| b == "subscriber") {
                    roles.push("subscriber".to_string());
                }
            }
            // Boolean flags
            if msg.is_moderator && !roles.contains(&"moderator".to_string()) {
                roles.push("moderator".to_string());
            }
            if msg.is_subscriber && !roles.contains(&"subscriber".to_string()) {
                roles.push("subscriber".to_string());
            }
        }
        _ => {}
    }

    // Everyone always has the "everyone" role
    roles.push("everyone".to_string());

    roles
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn command() -> SessionCommand {
        serde_json::from_value(json!({"id":"command-1", "permissionVersion":1,
            "trigger":"!clip", "action":"{\"FlowId\":\"flow-1\"}", "cooldown":null,
            "enabled":true, "allowedRoles":{"twitch":["moderator"]}}))
        .unwrap()
    }
    fn viewer() -> ChatMessage {
        serde_json::from_value(json!({"id":"message-1", "platform":"twitch", "user_id":"viewer-1",
            "user_name":"Alice", "user_display_name":"Alice", "message":"!clip hello", "timestamp":chrono::Utc::now().to_rfc3339(),
            "badges":[], "is_moderator":false, "is_subscriber":false, "is_vip":false, "platform_color":"", "emotes":null,"mentions":null})).unwrap()
    }
    #[test]
    fn explicit_empty_roles_deny_and_legacy_public_policy_is_preserved() {
        let mut command = command();
        command.allowed_roles = Some(HashMap::from([("twitch".into(), vec![])]));
        assert!(!check_command_permission(&command, &viewer(), "twitch"));
        assert!(!check_command_permission(&command, &viewer(), "youtube"));
        command.permission_version = 0;
        normalize_commands("session", std::slice::from_mut(&mut command));
        assert!(check_command_permission(&command, &viewer(), "twitch"));
        assert!(check_command_permission(&command, &viewer(), "youtube"));
        assert_eq!(command.permission_version, 1);
    }
    #[test]
    fn blacklist_wins_over_whitelist_and_provider_roles() {
        let mut command = command();
        assert!(!check_command_permission(&command, &viewer(), "twitch"));
        let mut moderator = viewer();
        moderator.is_moderator = true;
        assert!(check_command_permission(&command, &moderator, "twitch"));
        command.user_whitelist = Some(vec!["alice".into()]);
        assert!(check_command_permission(&command, &viewer(), "twitch"));
        command.user_blacklist = Some(vec!["ALICE".into()]);
        assert!(!check_command_permission(&command, &moderator, "twitch"));
    }
    #[test]
    fn blacklisted_user_id_survives_a_rename_and_whitelist_accepts_ids() {
        let mut command = command();
        let mut renamed = viewer();
        renamed.is_moderator = true;
        renamed.user_name = "NotAlice".into();
        renamed.user_display_name = "NotAlice".into();
        command.user_blacklist = Some(vec!["viewer-1".into()]);
        assert!(!check_command_permission(&command, &renamed, "twitch"));
        command.user_blacklist = Some(vec!["notalice".into()]);
        assert!(!check_command_permission(&command, &renamed, "twitch"));
        command.user_blacklist = Some(vec!["".into()]);
        assert!(check_command_permission(&command, &renamed, "twitch"));
        command.user_blacklist = None;
        command.user_whitelist = Some(vec!["VIEWER-1".into()]);
        assert!(check_command_permission(&command, &viewer(), "twitch"));
    }
    #[test]
    fn numeric_args_bind_whole_numbers_as_integers() {
        let mut command = command();
        command.input_bindings = [("seconds".to_string(), CommandInputSource::ArgsNumber)]
            .into_iter()
            .collect();
        let bind = |args: &str| {
            command_inputs(&command, &viewer(), args, "twitch")
                .map(|inputs| inputs["seconds"].clone())
        };
        assert_eq!(bind("600").unwrap(), json!(600));
        assert!(bind("600").unwrap().is_i64());
        assert!(bind("600.0").unwrap().is_i64());
        assert!(bind("-3").unwrap().is_i64());
        assert_eq!(bind("2.5").unwrap(), json!(2.5));
        assert!(bind("NaN").is_err());
        assert!(bind("abc").is_err());
    }
    #[test]
    fn command_tokens_and_definitions_are_validated() {
        assert_eq!(command_args("!clip", "!clip\t hello "), Some("hello"));
        assert_eq!(command_args("!clip", "!clip"), Some(""));
        assert_eq!(command_args("!clip", "!clipjunk"), None);
        assert_eq!(command_args("", "anything"), None);
        let mut command = command();
        assert!(validate_commands(std::slice::from_ref(&command)).is_ok());
        command.trigger.clear();
        assert!(validate_commands(std::slice::from_ref(&command)).is_err());
        command.trigger = "!clip".into();
        command.action = "{}".into();
        assert!(validate_commands(std::slice::from_ref(&command)).is_err());
    }
    #[test]
    fn substitution_preserves_json_and_never_reinterprets_user_placeholders() {
        let variables = HashMap::from([
            ("args".into(), "a\"b\\c\n{username}".into()),
            ("username".into(), "Alice".into()),
        ]);
        let action = substitute_action(r#"{"SystemControl":"{args}"}"#, &variables).unwrap();
        assert!(matches!(action, Action::SystemControl(value) if value == "a\"b\\c\n{username}"));
    }
    #[test]
    fn delivery_keys_are_scoped_and_require_real_provider_identity() {
        let identity = delivery_identity("twitch", 1, "session", "command", "message").unwrap();
        assert_eq!(
            identity,
            delivery_identity("twitch", 1, "session", "command", "message").unwrap()
        );
        assert_ne!(
            identity,
            delivery_identity("twitch", 2, "session", "command", "message").unwrap()
        );
        assert_ne!(
            identity,
            delivery_identity("twitch", 1, "session", "command-2", "message").unwrap()
        );
        assert!(delivery_identity("twitch", 1, "session", "command", "").is_err());
    }
    #[test]
    fn expired_history_and_future_commands_cannot_bypass_retention() {
        let now = chrono::Utc::now();
        assert!(validate_delivery_time(&now.to_rfc3339(), now).is_ok());
        assert!(
            validate_delivery_time(&(now - chrono::Duration::seconds(301)).to_rfc3339(), now)
                .is_err()
        );
        assert!(
            validate_delivery_time(&(now + chrono::Duration::seconds(121)).to_rfc3339(), now)
                .is_err()
        );
        assert!(validate_delivery_time("invalid", now).is_err());
    }
}
