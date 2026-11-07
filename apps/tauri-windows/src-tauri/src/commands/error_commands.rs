//! Frontend crash reporting — catches JS errors from the webview and shows a
//! native (non-webview) dialog. On "OK", reports the error to the VortiDeck
//! backend (which forwards to Discord/analytics server-side) + opens the logs.
//!
//! Paired with the Nuxt plugin `layers/desktop/plugins/0.error-reporter.ts`.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

/// Prevents dialog spam on cascading errors.
static CRASH_DIALOG_SHOWN: AtomicBool = AtomicBool::new(false);

/// The VortiDeck backend endpoint for error reports. The backend holds the
/// Discord webhook (or other forwarding) as a server-side secret — the app
/// only knows this public endpoint.
///
/// TODO (BACKEND — separate project, vortideck.com): implement this endpoint.
/// Reference implementation: `apps/web-app/src/workers/error-report.ts`.
/// The endpoint should: rate-limit (per IP), dedupe (by stack hash), store
/// (for analytics), and forward to Discord (webhook URL = server secret).
/// Until the endpoint exists, reports fail gracefully (logged warning, no crash).
const ERROR_REPORT_ENDPOINT: &str = "https://api.vortideck.com/v1/errors";

/// POST an error report to the VortiDeck backend (which forwards to Discord /
/// analytics server-side). Best-effort — failures are logged but not fatal.
/// Public so the timeout safety net (main.rs) can also report.
pub async fn report_to_backend(error: &serde_json::Value) -> Result<(), String> {
    let message = error
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown");
    let source = error
        .get("source")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");
    let stack = error.get("stack").and_then(|s| s.as_str()).unwrap_or("");

    reqwest::Client::new()
        .post(ERROR_REPORT_ENDPOINT)
        .json(&serde_json::json!({
            "message": message,
            "source": source,
            "stack": stack,
            "appVersion": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
        }))
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn report_frontend_crash(
    error: serde_json::Value,
    app: AppHandle,
    window: tauri::WebviewWindow,
) {
    let message = error
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("An unknown error occurred");
    let source = error
        .get("source")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");
    let stack = error.get("stack").and_then(|s| s.as_str()).unwrap_or("");

    // Always log the full error details to the log file.
    log::error!(
        "🔥 FRONTEND CRASH in window [{}]: {}",
        window.label(),
        message
    );
    log::error!("   Source: {}", source);
    if !stack.is_empty() {
        log::error!("   Stack:\n{}", stack);
    }

    // Reveal and focus ONLY the specific window that crashed
    let _ = window.show();
    let _ = window.set_focus();

    // Show the native dialog only once at a time (avoid spam on cascading errors).
    if CRASH_DIALOG_SHOWN.swap(true, Ordering::SeqCst) {
        return;
    }

    let log_dir = app.path().app_log_dir().ok();
    let error_for_report = error.clone();

    let dialog_msg = if cfg!(debug_assertions) {
        // Dev: full details + dev tools.
        format!(
            "{message}\n\n\
             Source: {source}\n\n\
             The window is now visible — press F12 for developer tools.\n\n\
             Click OK to report this to developers + open the logs folder,\n\
             or Cancel to dismiss."
        )
    } else {
        // Prod: user-friendly.
        format!(
            "{message}\n\n\
             An error occurred. The window is now visible.\n\n\
             Click OK to report this to the VortiDeck team + open the logs\n\
             folder (for support), or Cancel to dismiss."
        )
    };

    app.dialog()
        .message(dialog_msg)
        .title("VortiDeck — Error")
        .buttons(MessageDialogButtons::OkCancel)
        .show(move |ok| {
            // Reset dialog shown flag once user dismisses/acknowledges dialog
            CRASH_DIALOG_SHOWN.store(false, Ordering::SeqCst);

            if ok {
                let err = error_for_report;
                let dir = log_dir.clone();
                tauri::async_runtime::spawn(async move {
                    // Report to backend (best-effort — backend forwards to Discord).
                    match report_to_backend(&err).await {
                        Ok(()) => log::info!("✅ Error reported to developers via backend"),
                        Err(e) => log::warn!("⚠️ Failed to report error to backend: {}", e),
                    }
                    // Open logs folder.
                    if let Some(d) = &dir {
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("explorer").arg(d).spawn();
                        }
                        #[cfg(target_os = "macos")]
                        {
                            let _ = std::process::Command::new("open").arg(d).spawn();
                        }
                    }
                });
            }
        });
}
