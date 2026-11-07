/**
 * Language-agnostic schema for host function metadata
 */

#[derive(Debug, Clone)]
pub struct HostFunction {
    pub name: String,
    pub description: String,
    pub params: Vec<Parameter>,
    pub return_type: String,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub rust_type: String,
}

impl Parameter {
    /// Convert Rust type to language-specific type
    pub fn to_language_type(&self, language: &str) -> String {
        match language {
            "typescript" => rust_to_typescript(&self.rust_type),
            "python" => rust_to_python(&self.rust_type),
            "go" => rust_to_go(&self.rust_type),
            _ => "any".to_string(),
        }
    }
}

/// Extract all #[vortideck_host] annotated functions from Rust source
pub fn extract_host_functions(content: &str) -> Vec<HostFunction> {
    let mut functions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Look for #[vortideck_host] annotation
        if line == "#[vortideck_host]" {
            // Extract doc comment (previous lines starting with ///)
            let mut description = String::new();
            let mut j = i.saturating_sub(1);
            while j > 0 {
                let doc_line = lines[j].trim();
                if doc_line.starts_with("///") {
                    description =
                        doc_line.trim_start_matches("///").trim().to_string() + " " + &description;
                    j -= 1;
                } else if doc_line.starts_with("//") && !doc_line.starts_with("///") {
                    // Regular comment, also extract
                    description =
                        doc_line.trim_start_matches("//").trim().to_string() + " " + &description;
                    j -= 1;
                } else {
                    break;
                }
            }

            // Parse function signature (next line)
            if i + 1 < lines.len() {
                let fn_line = lines[i + 1].trim();
                if let Some(func) = parse_function_signature(fn_line, description.trim()) {
                    functions.push(func);
                }
            }
        }

        i += 1;
    }

    functions
}

fn parse_function_signature(line: &str, description: &str) -> Option<HostFunction> {
    // Parse: fn function_name(param1: Type1, param2: Type2) -> Result<ReturnType, String>
    if !line.starts_with("fn ") {
        return None;
    }

    let line = line.trim_start_matches("fn ").trim();
    let paren_pos = line.find('(')?;
    let name = line[..paren_pos].trim().to_string();

    let close_paren = line.rfind(')')?;
    let params_str = &line[paren_pos + 1..close_paren];

    let mut params = Vec::new();
    if !params_str.is_empty() {
        for param in params_str.split(',') {
            let param = param.trim();
            if let Some(colon_pos) = param.find(':') {
                let param_name = param[..colon_pos].trim().to_string();
                let rust_type = param[colon_pos + 1..].trim().to_string();

                params.push(Parameter {
                    name: param_name,
                    rust_type,
                });
            }
        }
    }

    // Extract return type
    let return_type = if let Some(arrow_pos) = line.find("->") {
        let ret_str = line[arrow_pos + 2..].trim();
        // Handle Result<T, E>
        if ret_str.starts_with("Result<") {
            let inner = ret_str.trim_start_matches("Result<");
            let comma_pos = inner.find(',')?;
            let success_type = inner[..comma_pos].trim();
            success_type.to_string()
        } else {
            ret_str.to_string()
        }
    } else {
        "()".to_string()
    };

    Some(HostFunction {
        name,
        description: description.to_string(),
        params,
        return_type,
    })
}

// Type conversion functions

fn rust_to_typescript(rust_type: &str) -> String {
    match rust_type {
        "String" | "&str" => "string".to_string(),
        "bool" => "boolean".to_string(),
        "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "usize" => "number".to_string(),
        "()" => "void".to_string(),
        s if s.starts_with("Vec<") => {
            let inner = s.trim_start_matches("Vec<").trim_end_matches('>');
            format!("{}[]", rust_to_typescript(inner))
        }
        s if s.starts_with("Option<") => {
            let inner = s.trim_start_matches("Option<").trim_end_matches('>');
            format!("{} | null", rust_to_typescript(inner))
        }
        _ => "any".to_string(),
    }
}

fn rust_to_python(rust_type: &str) -> String {
    match rust_type {
        "String" | "&str" => "str".to_string(),
        "bool" => "bool".to_string(),
        "i32" | "i64" | "u32" | "u64" | "usize" => "int".to_string(),
        "f32" | "f64" => "float".to_string(),
        "()" => "None".to_string(),
        s if s.starts_with("Vec<") => {
            let inner = s.trim_start_matches("Vec<").trim_end_matches('>');
            format!("list[{}]", rust_to_python(inner))
        }
        s if s.starts_with("Option<") => {
            let inner = s.trim_start_matches("Option<").trim_end_matches('>');
            format!("{} | None", rust_to_python(inner))
        }
        _ => "Any".to_string(),
    }
}

fn rust_to_go(rust_type: &str) -> String {
    match rust_type {
        "String" | "&str" => "string".to_string(),
        "bool" => "bool".to_string(),
        "i32" | "u32" => "int32".to_string(),
        "i64" | "u64" => "int64".to_string(),
        "f32" => "float32".to_string(),
        "f64" => "float64".to_string(),
        "usize" => "uint".to_string(),
        "()" => "void".to_string(),
        s if s.starts_with("Vec<") => {
            let inner = s.trim_start_matches("Vec<").trim_end_matches('>');
            format!("[]{}", rust_to_go(inner))
        }
        s if s.starts_with("Option<") => {
            let inner = s.trim_start_matches("Option<").trim_end_matches('>');
            format!("*{}", rust_to_go(inner))
        }
        _ => "interface{}".to_string(),
    }
}
