/**
 * Console Logger — intercepts console.error/warn and forwards them to the
 * Tauri log plugin (@tauri-apps/plugin-log), so frontend errors appear in the
 * log file alongside Rust logs.
 *
 * Loads after the error-reporter (prefix `1.`). Only intercepts error/warn
 * (not log/debug/info — too noisy + too much IPC overhead). Deduplicates
 * repeated messages (5s window) to prevent render-loop spam.
 *
 * Layer: `layers/desktop` (Tauri desktop — applies to ALL desktop windows).
 * Mobile: would need a copy in `layers/mobile/plugins/` (separate Nuxt app).
 */
import { error as tauriError, warn as tauriWarn } from "@tauri-apps/plugin-log";

// Dedup: only forward each unique message once per 5 seconds (prevents
// render-loop error spam from flooding the log file + IPC).
const recentMessages = new Map<string, number>();
const DEDUP_WINDOW = 5000;

function shouldLog(message: string): boolean {
  const now = Date.now();
  const last = recentMessages.get(message);
  if (last !== undefined && now - last < DEDUP_WINDOW) return false;
  recentMessages.set(message, now);
  // Prune old entries (prevent the Map from growing forever).
  if (recentMessages.size > 100) {
    for (const [key, time] of recentMessages) {
      if (now - time > DEDUP_WINDOW) recentMessages.delete(key);
    }
  }
  return true;
}

function safeStringify(args: any[]): string {
  return args
    .map((arg) => {
      if (typeof arg === "string") return arg;
      if (arg instanceof Error) return arg.stack || arg.message;
      try {
        return JSON.stringify(arg);
      } catch {
        return String(arg);
      }
    })
    .join(" ");
}

export default defineNuxtPlugin(() => {
  // Intercept console.error → Tauri log file (via @tauri-apps/plugin-log IPC).
  const origError = console.error;
  console.error = (...args: any[]) => {
    const msg = safeStringify(args);
    if (shouldLog(msg)) {
      tauriError(`[frontend] ${msg}`).catch(() => {});
    }
    origError.apply(console, args); // preserve browser console output
  };

  // Intercept console.warn → Tauri log file.
  const origWarn = console.warn;
  console.warn = (...args: any[]) => {
    const msg = safeStringify(args);
    if (shouldLog(msg)) {
      tauriWarn(`[frontend] ${msg}`).catch(() => {});
    }
    origWarn.apply(console, args);
  };

  // NOTE: console.log/debug/info are NOT intercepted (too noisy + IPC overhead).
  // Only errors and warnings go to the log file.
});
