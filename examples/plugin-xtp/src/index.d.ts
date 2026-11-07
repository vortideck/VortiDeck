declare module 'main' {
  export function execute(): I32;
  export function get_metadata(): I32;
  export function register(): I32;
}

declare module 'extism:host' {
  interface user {
  vortideck_get_context(): I64;
  vortideck_log(ptr: I64): I64;
  vortideck_register_node(ptr: I64): I64;
  }
}

