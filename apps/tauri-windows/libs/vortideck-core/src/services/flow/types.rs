use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Type system for flow nodes - ensures type safety across connections
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FlowDataType {
    /// Any type - accepts anything (use sparingly)
    Any,

    /// Primitive types
    String {
        #[serde(skip_serializing_if = "Option::is_none")]
        pattern: Option<String>, // Regex pattern for validation
        #[serde(skip_serializing_if = "Option::is_none")]
        min_length: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_length: Option<usize>,
    },
    Number {
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        integer_only: Option<bool>,
    },
    Boolean,
    Null,

    /// Complex types
    Array {
        item_type: Box<FlowDataType>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_items: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_items: Option<usize>,
    },
    Object {
        properties: HashMap<String, FlowDataType>,
        #[serde(default)]
        required: Vec<String>,
        #[serde(default)]
        additional_properties: bool,
    },

    /// Union type - value can be one of multiple types
    Union {
        types: Vec<FlowDataType>,
    },

    /// Domain-specific types
    File {
        #[serde(skip_serializing_if = "Option::is_none")]
        accepted_types: Option<Vec<String>>, // e.g., [".mp3", ".wav"]
        #[serde(skip_serializing_if = "Option::is_none")]
        max_size_bytes: Option<u64>,
    },
    DateTime {
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<String>, // ISO8601, RFC3339, etc.
    },
    Duration {
        #[serde(skip_serializing_if = "Option::is_none")]
        unit: Option<DurationUnit>,
    },
    Url {
        #[serde(skip_serializing_if = "Option::is_none")]
        schemes: Option<Vec<String>>, // ["http", "https", "file"]
    },
    Email,
    Json,
    Binary,

    /// AI/LLM specific types
    Prompt {
        #[serde(skip_serializing_if = "Option::is_none")]
        max_tokens: Option<u32>,
    },
    ChatMessage,
    ModelOutput,
    Embedding {
        dimensions: usize,
    },

    /// Flow control types
    Trigger,
    Event {
        event_type: String,
    },
    Signal,

    /// Custom type with validator
    Custom {
        type_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        schema: Option<Value>, // JSON Schema
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DurationUnit {
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl FlowDataType {
    /// Check if a value matches this type
    pub fn validate(&self, value: &Value) -> Result<(), TypeError> {
        match self {
            FlowDataType::Any => Ok(()),

            FlowDataType::String {
                pattern,
                min_length,
                max_length,
            } => {
                let s = value.as_str().ok_or(TypeError::WrongType {
                    expected: "string".to_string(),
                    got: type_name_of(value),
                })?;

                if let Some(min) = min_length {
                    if s.len() < *min {
                        return Err(TypeError::StringTooShort {
                            min: *min,
                            got: s.len(),
                        });
                    }
                }

                if let Some(max) = max_length {
                    if s.len() > *max {
                        return Err(TypeError::StringTooLong {
                            max: *max,
                            got: s.len(),
                        });
                    }
                }

                if let Some(pat) = pattern {
                    let re = regex::Regex::new(pat)
                        .map_err(|e| TypeError::InvalidPattern(e.to_string()))?;
                    if !re.is_match(s) {
                        return Err(TypeError::PatternMismatch {
                            pattern: pat.clone(),
                        });
                    }
                }

                Ok(())
            }

            FlowDataType::Number {
                min,
                max,
                integer_only,
            } => {
                let num = value.as_f64().ok_or(TypeError::WrongType {
                    expected: "number".to_string(),
                    got: type_name_of(value),
                })?;

                if let Some(true) = integer_only {
                    if num.fract() != 0.0 {
                        return Err(TypeError::NotInteger);
                    }
                }

                if let Some(min_val) = min {
                    if num < *min_val {
                        return Err(TypeError::NumberTooSmall {
                            min: *min_val,
                            got: num,
                        });
                    }
                }

                if let Some(max_val) = max {
                    if num > *max_val {
                        return Err(TypeError::NumberTooLarge {
                            max: *max_val,
                            got: num,
                        });
                    }
                }

                Ok(())
            }

            FlowDataType::Boolean => {
                value.as_bool().ok_or(TypeError::WrongType {
                    expected: "boolean".to_string(),
                    got: type_name_of(value),
                })?;
                Ok(())
            }

            FlowDataType::Null => {
                if !value.is_null() {
                    return Err(TypeError::WrongType {
                        expected: "null".to_string(),
                        got: type_name_of(value),
                    });
                }
                Ok(())
            }

            FlowDataType::Array {
                item_type,
                min_items,
                max_items,
            } => {
                let arr = value.as_array().ok_or(TypeError::WrongType {
                    expected: "array".to_string(),
                    got: type_name_of(value),
                })?;

                if let Some(min) = min_items {
                    if arr.len() < *min {
                        return Err(TypeError::ArrayTooShort {
                            min: *min,
                            got: arr.len(),
                        });
                    }
                }

                if let Some(max) = max_items {
                    if arr.len() > *max {
                        return Err(TypeError::ArrayTooLong {
                            max: *max,
                            got: arr.len(),
                        });
                    }
                }

                // Validate each item
                for (i, item) in arr.iter().enumerate() {
                    item_type
                        .validate(item)
                        .map_err(|e| TypeError::ArrayItemError {
                            index: i,
                            error: Box::new(e),
                        })?;
                }

                Ok(())
            }

            FlowDataType::Object {
                properties,
                required,
                additional_properties,
            } => {
                let obj = value.as_object().ok_or(TypeError::WrongType {
                    expected: "object".to_string(),
                    got: type_name_of(value),
                })?;

                // Check required properties
                for req in required {
                    if !obj.contains_key(req) {
                        return Err(TypeError::MissingProperty {
                            property: req.clone(),
                        });
                    }
                }

                // Validate each property
                for (key, val) in obj {
                    if let Some(prop_type) = properties.get(key) {
                        prop_type
                            .validate(val)
                            .map_err(|e| TypeError::PropertyError {
                                property: key.clone(),
                                error: Box::new(e),
                            })?;
                    } else if !additional_properties {
                        return Err(TypeError::UnexpectedProperty {
                            property: key.clone(),
                        });
                    }
                }

                Ok(())
            }

            FlowDataType::Union { types } => {
                // Try each type until one succeeds
                for t in types {
                    if t.validate(value).is_ok() {
                        return Ok(());
                    }
                }

                Err(TypeError::NoUnionMatch { tried: types.len() })
            }

            FlowDataType::File {
                accepted_types,
                max_size_bytes,
            } => {
                // For files, we expect a string path or object with path
                let file_path = if let Some(s) = value.as_str() {
                    s.to_string()
                } else if let Some(obj) = value.as_object() {
                    obj.get("path")
                        .and_then(|v| v.as_str())
                        .ok_or(TypeError::WrongType {
                            expected: "file path or {path: string}".to_string(),
                            got: type_name_of(value),
                        })?
                        .to_string()
                } else {
                    return Err(TypeError::WrongType {
                        expected: "file path or {path: string}".to_string(),
                        got: type_name_of(value),
                    });
                };

                // Check file extension
                if let Some(accepted) = accepted_types {
                    let has_valid_ext = accepted.iter().any(|ext| file_path.ends_with(ext));
                    if !has_valid_ext {
                        return Err(TypeError::InvalidFileType {
                            accepted: accepted.clone(),
                            got: file_path.clone(),
                        });
                    }
                }

                // Check file size if we have size info
                if let Some(max_size) = max_size_bytes {
                    if let Some(obj) = value.as_object() {
                        if let Some(size_val) = obj.get("size") {
                            let size = size_val.as_u64().unwrap_or(0);
                            if size > *max_size {
                                return Err(TypeError::FileTooLarge {
                                    max: *max_size,
                                    got: size,
                                });
                            }
                        }
                    }
                }

                Ok(())
            }

            FlowDataType::DateTime { .. } => {
                // Accept string and try to parse as ISO8601
                let s = value.as_str().ok_or(TypeError::WrongType {
                    expected: "datetime string".to_string(),
                    got: type_name_of(value),
                })?;

                chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(|e| TypeError::InvalidDateTime(e.to_string()))?;

                Ok(())
            }

            FlowDataType::Url { schemes } => {
                let s = value.as_str().ok_or(TypeError::WrongType {
                    expected: "url string".to_string(),
                    got: type_name_of(value),
                })?;

                let url = url::Url::parse(s).map_err(|e| TypeError::InvalidUrl(e.to_string()))?;

                if let Some(allowed_schemes) = schemes {
                    if !allowed_schemes.contains(&url.scheme().to_string()) {
                        return Err(TypeError::InvalidUrlScheme {
                            expected: allowed_schemes.clone(),
                            got: url.scheme().to_string(),
                        });
                    }
                }

                Ok(())
            }

            FlowDataType::Email => {
                let s = value.as_str().ok_or(TypeError::WrongType {
                    expected: "email string".to_string(),
                    got: type_name_of(value),
                })?;

                // Simple email validation
                if !s.contains('@') || !s.contains('.') {
                    return Err(TypeError::InvalidEmail);
                }

                Ok(())
            }

            FlowDataType::Json => {
                // Any valid JSON value is acceptable
                Ok(())
            }

            // For other types, just do basic validation
            _ => Ok(()),
        }
    }

    /// Check if this type is compatible with another type (for connections)
    pub fn is_compatible_with(&self, other: &FlowDataType) -> bool {
        // Any type accepts everything
        if matches!(other, FlowDataType::Any) {
            return true;
        }

        // Exact match
        if self == other {
            return true;
        }

        // Union types
        if let FlowDataType::Union { types } = self {
            return types.iter().any(|t| t.is_compatible_with(other));
        }

        if let FlowDataType::Union { types } = other {
            return types.iter().any(|t| self.is_compatible_with(t));
        }

        // String can connect to Email, Url, DateTime, etc.
        match (self, other) {
            (FlowDataType::String { .. }, FlowDataType::Email) => true,
            (FlowDataType::String { .. }, FlowDataType::Url { .. }) => true,
            (FlowDataType::String { .. }, FlowDataType::DateTime { .. }) => true,
            (FlowDataType::String { .. }, FlowDataType::Json) => true,
            (FlowDataType::Number { .. }, FlowDataType::Duration { .. }) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TypeError {
    WrongType {
        expected: String,
        got: String,
    },
    StringTooShort {
        min: usize,
        got: usize,
    },
    StringTooLong {
        max: usize,
        got: usize,
    },
    PatternMismatch {
        pattern: String,
    },
    InvalidPattern(String),
    NumberTooSmall {
        min: f64,
        got: f64,
    },
    NumberTooLarge {
        max: f64,
        got: f64,
    },
    NotInteger,
    ArrayTooShort {
        min: usize,
        got: usize,
    },
    ArrayTooLong {
        max: usize,
        got: usize,
    },
    ArrayItemError {
        index: usize,
        error: Box<TypeError>,
    },
    MissingProperty {
        property: String,
    },
    PropertyError {
        property: String,
        error: Box<TypeError>,
    },
    UnexpectedProperty {
        property: String,
    },
    NoUnionMatch {
        tried: usize,
    },
    InvalidFileType {
        accepted: Vec<String>,
        got: String,
    },
    FileTooLarge {
        max: u64,
        got: u64,
    },
    InvalidDateTime(String),
    InvalidUrl(String),
    InvalidUrlScheme {
        expected: Vec<String>,
        got: String,
    },
    InvalidEmail,
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::WrongType { expected, got } => {
                write!(f, "Expected {}, got {}", expected, got)
            }
            TypeError::StringTooShort { min, got } => {
                write!(
                    f,
                    "String too short: minimum {} characters, got {}",
                    min, got
                )
            }
            TypeError::StringTooLong { max, got } => {
                write!(
                    f,
                    "String too long: maximum {} characters, got {}",
                    max, got
                )
            }
            TypeError::PatternMismatch { pattern } => {
                write!(f, "String does not match pattern: {}", pattern)
            }
            TypeError::InvalidPattern(err) => {
                write!(f, "Invalid regex pattern: {}", err)
            }
            TypeError::NumberTooSmall { min, got } => {
                write!(f, "Number too small: minimum {}, got {}", min, got)
            }
            TypeError::NumberTooLarge { max, got } => {
                write!(f, "Number too large: maximum {}, got {}", max, got)
            }
            TypeError::NotInteger => {
                write!(f, "Expected integer, got decimal number")
            }
            TypeError::ArrayTooShort { min, got } => {
                write!(f, "Array too short: minimum {} items, got {}", min, got)
            }
            TypeError::ArrayTooLong { max, got } => {
                write!(f, "Array too long: maximum {} items, got {}", max, got)
            }
            TypeError::ArrayItemError { index, error } => {
                write!(f, "Array item [{}]: {}", index, error)
            }
            TypeError::MissingProperty { property } => {
                write!(f, "Missing required property: {}", property)
            }
            TypeError::PropertyError { property, error } => {
                write!(f, "Property '{}': {}", property, error)
            }
            TypeError::UnexpectedProperty { property } => {
                write!(f, "Unexpected property: {}", property)
            }
            TypeError::NoUnionMatch { tried } => {
                write!(f, "Value does not match any of {} union types", tried)
            }
            TypeError::InvalidFileType { accepted, got } => {
                write!(
                    f,
                    "Invalid file type: expected one of {:?}, got {}",
                    accepted, got
                )
            }
            TypeError::FileTooLarge { max, got } => {
                write!(
                    f,
                    "File too large: maximum {} bytes, got {} bytes",
                    max, got
                )
            }
            TypeError::InvalidDateTime(err) => {
                write!(f, "Invalid datetime: {}", err)
            }
            TypeError::InvalidUrl(err) => {
                write!(f, "Invalid URL: {}", err)
            }
            TypeError::InvalidUrlScheme { expected, got } => {
                write!(
                    f,
                    "Invalid URL scheme: expected one of {:?}, got {}",
                    expected, got
                )
            }
            TypeError::InvalidEmail => {
                write!(f, "Invalid email address")
            }
        }
    }
}

impl std::error::Error for TypeError {}

/// Get the type name of a JSON value
fn type_name_of(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(_) => "number".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_string_validation() {
        let str_type = FlowDataType::String {
            pattern: Some(r"^\d{3}-\d{3}-\d{4}$".to_string()),
            min_length: Some(12),
            max_length: Some(12),
        };

        assert!(str_type.validate(&json!("123-456-7890")).is_ok());
        assert!(str_type.validate(&json!("abc")).is_err());
        assert!(str_type.validate(&json!(123)).is_err());
    }

    #[test]
    fn test_number_validation() {
        let num_type = FlowDataType::Number {
            min: Some(0.0),
            max: Some(100.0),
            integer_only: Some(true),
        };

        assert!(num_type.validate(&json!(50)).is_ok());
        assert!(num_type.validate(&json!(50.5)).is_err()); // Not integer
        assert!(num_type.validate(&json!(150)).is_err()); // Too large
    }

    #[test]
    fn test_array_validation() {
        let arr_type = FlowDataType::Array {
            item_type: Box::new(FlowDataType::Number {
                min: None,
                max: None,
                integer_only: None,
            }),
            min_items: Some(2),
            max_items: Some(5),
        };

        assert!(arr_type.validate(&json!([1, 2, 3])).is_ok());
        assert!(arr_type.validate(&json!([1])).is_err()); // Too short
        assert!(arr_type.validate(&json!([1, "a", 3])).is_err()); // Wrong item type
    }

    #[test]
    fn test_type_compatibility() {
        let str_type = FlowDataType::String {
            pattern: None,
            min_length: None,
            max_length: None,
        };
        let email_type = FlowDataType::Email;
        let any_type = FlowDataType::Any;

        assert!(str_type.is_compatible_with(&email_type));
        assert!(email_type.is_compatible_with(&any_type));
        assert!(str_type.is_compatible_with(&any_type));
    }
}
