/**
 * Error Reporter — catches frontend JS errors and reports them to the backend
 * (native dialog). Loads FIRST (prefix `0.`). No timeout — the error reports
 * itself the instant it happens.
 */
import { invoke } from "@tauri-apps/api/core";

export default defineNuxtPlugin(() => {
  function report(message: string, source: string, stack: string) {
    invoke("report_frontend_crash", { error: { message, source, stack } }).catch(() => {});
  }

  window.addEventListener("error", (event) => {
    report(
      event.message || "Unknown error",
      event.filename ? `${event.filename}:${event.lineno}:${event.colno}` : "unknown",
      event.error?.stack || "",
    );
  });

  window.addEventListener("unhandledrejection", (event) => {
    report(
      `Unhandled rejection: ${event.reason?.message || event.reason || "unknown"}`,
      "promise",
      event.reason?.stack || "",
    );
  });
});
