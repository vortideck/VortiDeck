/**
 * Extism Plugin Type Definitions
 *
 * AUTO-GENERATED from Rust host functions
 * DO NOT EDIT MANUALLY
 *
 * Note: Use Extism integer types (I32, I64) for compatibility with extism-js compiler
 */

declare module 'main' {
  export function execute(): I32;
  export function get_metadata(): I32;
  export function register(): I32;
}

declare module 'extism:host' {
  interface user {
    vortideck_log(ptr: I64): I64;
    vortideck_register_node(ptr: I64): I64;
    vortideck_get_context(): I64;
    vortideck_emit_progress(ptr: I64): I64;
    vortideck_emit_event(ptr: I64): I64;
    vortideck_storage_get(ptr: I64): I64;
    vortideck_storage_set(ptr: I64): I64;
    vortideck_read_file(ptr: I64): I64;
    vortideck_write_file(ptr: I64): I64;
    vortideck_http_request(ptr: I64): I64;
  }
}
