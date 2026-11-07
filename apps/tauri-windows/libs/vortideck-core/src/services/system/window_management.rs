/// Window management functionality for focusing external windows
/// This module provides platform-specific window focus capabilities

#[cfg(target_os = "windows")]
use windows::{
    core::{BOOL, PCWSTR},
    Win32::Foundation::{CloseHandle, HWND, LPARAM},
    Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_NATIVE, PROCESS_QUERY_INFORMATION,
        PROCESS_VM_READ,
    },
    Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
        SetForegroundWindow, ShowWindow, SW_RESTORE,
    },
};

#[cfg(target_os = "windows")]
use regex::{Regex, RegexBuilder};
#[cfg(target_os = "windows")]
use std::collections::HashSet;
#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStringExt;

/// Result type for window operations
pub type WindowResult<T> = Result<T, WindowError>;

/// Errors that can occur during window operations
#[derive(Debug)]
pub enum WindowError {
    WindowNotFound(String),
    AccessDenied(String),
    PlatformNotSupported,
    SystemError(String),
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowError::WindowNotFound(msg) => write!(f, "Window not found: {}", msg),
            WindowError::AccessDenied(msg) => write!(f, "Access denied: {}", msg),
            WindowError::PlatformNotSupported => write!(f, "Platform not supported"),
            WindowError::SystemError(msg) => write!(f, "System error: {}", msg),
        }
    }
}

impl std::error::Error for WindowError {}

/// Focus a window by matching its title and/or process name
///
/// # Arguments
/// * `window_title` - The title hint to match (supports partial / dynamic title matching)
/// * `process_name` - The process name (with or without .exe). Optional when title is provided.
/// * `match_mode` - Matching strategy: smart | exact | contains | regex | process_only
///
/// # Returns
/// * `Ok(())` if the window was successfully focused
/// * `Err(WindowError)` if the window could not be found or focused
pub fn focus_window(window_title: &str, process_name: &str, match_mode: &str) -> WindowResult<()> {
    #[cfg(target_os = "windows")]
    {
        focus_window_windows(window_title, process_name, match_mode)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(WindowError::PlatformNotSupported)
    }
}

#[cfg(target_os = "windows")]
fn focus_window_windows(
    window_title: &str,
    process_name: &str,
    match_mode: &str,
) -> WindowResult<()> {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum WindowMatchMode {
        Smart,
        Exact,
        Contains,
        Regex,
        ProcessOnly,
    }

    impl WindowMatchMode {
        fn parse(value: &str) -> Self {
            match value.trim().to_lowercase().as_str() {
                "exact" => Self::Exact,
                "contains" => Self::Contains,
                "regex" => Self::Regex,
                "process_only" => Self::ProcessOnly,
                _ => Self::Smart,
            }
        }
    }

    #[derive(Debug, Clone)]
    struct CandidateWindow {
        hwnd: HWND,
        title: String,
        process_name: String,
    }

    #[derive(Debug, Clone)]
    struct TargetMatcher {
        title_raw: String,
        title_norm: String,
        process_norm: String,
        title_hints: Vec<String>,
        title_tokens: HashSet<String>,
        mode: WindowMatchMode,
        regex: Option<Regex>,
    }

    fn normalize_process_name(name: &str) -> String {
        name.trim().trim_end_matches(".exe").to_lowercase()
    }

    fn normalize_title(title: &str) -> String {
        title.trim().to_lowercase()
    }

    fn extract_title_hints(title: &str) -> Vec<String> {
        // Common desktop title separators: "<doc> - <app>", "<page> | <app>", etc.
        let separators = [" - ", " — ", " | ", " · ", ": "];
        let mut hints = Vec::new();
        let mut push_hint = |value: &str| {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                hints.push(trimmed.to_lowercase());
            }
        };

        push_hint(title);
        for sep in separators {
            if let Some((left, right)) = title.rsplit_once(sep) {
                push_hint(left);
                push_hint(right);
            }
        }

        // De-duplicate while preserving insertion order.
        let mut seen = HashSet::new();
        hints
            .into_iter()
            .filter(|item| seen.insert(item.clone()))
            .collect()
    }

    fn tokenize_title(title: &str) -> HashSet<String> {
        title
            .split(|c: char| !c.is_alphanumeric())
            .map(|p| p.trim().to_lowercase())
            .filter(|p| p.len() >= 3)
            .collect::<HashSet<_>>()
    }

    fn score_candidate(target: &TargetMatcher, candidate: &CandidateWindow) -> Option<i32> {
        let cand_process = normalize_process_name(&candidate.process_name);
        let cand_title = normalize_title(&candidate.title);

        if target.title_norm.is_empty() && target.process_norm.is_empty() {
            return None;
        }

        let process_matches = target.process_norm.is_empty() || cand_process == target.process_norm;

        match target.mode {
            WindowMatchMode::ProcessOnly => {
                if !target.process_norm.is_empty() && cand_process == target.process_norm {
                    return Some(160);
                }
                None
            }
            WindowMatchMode::Exact => {
                if !process_matches {
                    return None;
                }
                if target.title_norm.is_empty() || cand_title == target.title_norm {
                    return Some(if target.title_norm.is_empty() {
                        140
                    } else {
                        230
                    });
                }
                None
            }
            WindowMatchMode::Contains => {
                if !process_matches {
                    return None;
                }
                if target.title_norm.is_empty()
                    || cand_title.contains(&target.title_norm)
                    || target.title_norm.contains(&cand_title)
                {
                    return Some(if target.title_norm.is_empty() {
                        135
                    } else {
                        210
                    });
                }
                None
            }
            WindowMatchMode::Regex => {
                if !process_matches {
                    return None;
                }
                let re = target.regex.as_ref()?;
                if re.is_match(&candidate.title) {
                    return Some(205);
                }
                None
            }
            WindowMatchMode::Smart => {
                let process_score = if target.process_norm.is_empty() {
                    0
                } else if cand_process == target.process_norm {
                    120
                } else {
                    return None;
                };

                let title_score = if target.title_norm.is_empty() {
                    0
                } else if cand_title == target.title_norm {
                    110
                } else if cand_title.contains(&target.title_norm)
                    || target.title_norm.contains(&cand_title)
                {
                    85
                } else if target.title_hints.iter().any(|hint| {
                    hint.len() >= 3 && (cand_title.contains(hint) || hint.contains(&cand_title))
                }) {
                    64
                } else {
                    let cand_tokens = tokenize_title(&cand_title);
                    let overlap = cand_tokens.intersection(&target.title_tokens).count();
                    if overlap >= 2 {
                        48
                    } else {
                        return None;
                    }
                };

                Some(process_score + title_score)
            }
        }
    }

    unsafe {
        let parsed_mode = WindowMatchMode::parse(match_mode);
        let title_trimmed = window_title.trim();
        let regex = if parsed_mode == WindowMatchMode::Regex && !title_trimmed.is_empty() {
            Some(
                RegexBuilder::new(title_trimmed)
                    .case_insensitive(true)
                    .build()
                    .map_err(|e| {
                        WindowError::SystemError(format!(
                            "Invalid regex pattern '{}': {}",
                            title_trimmed, e
                        ))
                    })?,
            )
        } else {
            None
        };

        let target = TargetMatcher {
            title_raw: title_trimmed.to_string(),
            title_norm: normalize_title(window_title),
            process_norm: normalize_process_name(process_name),
            title_hints: extract_title_hints(window_title),
            title_tokens: tokenize_title(window_title),
            mode: parsed_mode,
            regex,
        };

        if target.mode == WindowMatchMode::ProcessOnly && target.process_norm.is_empty() {
            return Err(WindowError::SystemError(
                "Process-only match mode requires a process name".to_string(),
            ));
        }

        if target.title_norm.is_empty() && target.process_norm.is_empty() {
            return Err(WindowError::SystemError(
                "Focus window requires at least a window title or a process name".to_string(),
            ));
        }

        let mut candidates: Vec<CandidateWindow> = Vec::new();

        // Callback to enumerate windows
        unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            unsafe {
                let windows = &mut *(lparam.0 as *mut Vec<CandidateWindow>);

                // Check if window is visible
                if IsWindowVisible(hwnd).as_bool() {
                    // Get window title
                    let mut title: [u16; 512] = [0; 512];
                    let len = GetWindowTextW(hwnd, &mut title);
                    if len > 0 {
                        let window_title = OsString::from_wide(&title[..len as usize])
                            .to_string_lossy()
                            .into_owned();

                        // Get process name
                        let mut process_id: u32 = 0;
                        GetWindowThreadProcessId(hwnd, Some(&mut process_id as *mut u32));

                        if let Ok(handle) = OpenProcess(
                            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                            false,
                            process_id,
                        ) {
                            let mut process_name_buf: [u16; 512] = [0; 512];
                            let mut size = 512u32;
                            if QueryFullProcessImageNameW(
                                handle,
                                PROCESS_NAME_NATIVE,
                                windows::core::PWSTR(process_name_buf.as_mut_ptr()),
                                &mut size as *mut u32,
                            )
                            .is_ok()
                            {
                                let full_path =
                                    OsString::from_wide(&process_name_buf[..size as usize])
                                        .to_string_lossy()
                                        .into_owned();
                                let process_name = std::path::Path::new(&full_path)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("")
                                    .to_string();
                                windows.push(CandidateWindow {
                                    hwnd,
                                    title: window_title,
                                    process_name,
                                });
                            }
                            let _ = CloseHandle(handle);
                        }
                    }
                }

                BOOL(1) // Continue enumeration
            }
        }

        let lparam = LPARAM(&mut candidates as *mut _ as isize);

        if EnumWindows(Some(enum_callback), lparam).is_err() {
            return Err(WindowError::SystemError(
                "Failed to enumerate visible windows".to_string(),
            ));
        }

        let best = candidates
            .iter()
            .filter_map(|cand| score_candidate(&target, cand).map(|score| (score, cand)))
            .max_by_key(|(score, _)| *score);

        if let Some((score, selected)) = best {
            println!(
                "✅ Selected window candidate (score={}): {} [{}]",
                score, selected.title, selected.process_name
            );
            // Restore window if minimized
            ShowWindow(selected.hwnd, SW_RESTORE);
            // Bring window to foreground
            if SetForegroundWindow(selected.hwnd).as_bool() {
                println!(
                    "✅ Successfully focused window: {} [{}]",
                    window_title, process_name
                );
                Ok(())
            } else {
                println!("⚠️  SetForegroundWindow returned false, window may still be focused");
                Ok(()) // Still consider it a success
            }
        } else {
            let requested = if target.process_norm.is_empty() {
                format!("title hint '{}'", target.title_raw)
            } else if target.title_norm.is_empty() {
                format!("process '{}'", target.process_norm)
            } else {
                format!(
                    "title hint '{}' + process '{}'",
                    target.title_raw, target.process_norm
                )
            };
            Err(WindowError::WindowNotFound(format!(
                "No visible window matched {} using '{}' mode.",
                requested, match_mode
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_focus_window_format() {
        // This test just ensures the function signature is correct
        // Actual testing would require a real window to be open
        let result = focus_window("Notepad", "notepad", "smart");
        // We don't assert anything because we don't know if Notepad is running
        println!("Result: {:?}", result);
    }
}
