use std::fmt;

/// Common utility functions for the VortiDeck core library

/// Format an error message with context
pub fn format_error(context: &str, error: &impl fmt::Display) -> String {
    format!("{}: {}", context, error)
}

/// Convert a string to a valid identifier
pub fn to_identifier(s: &str) -> String {
    s.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "_")
        .trim_matches('_')
        .to_string()
}

/// Validate a string is a valid identifier
pub fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Convert a string to a display name
pub fn to_display_name(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
} 