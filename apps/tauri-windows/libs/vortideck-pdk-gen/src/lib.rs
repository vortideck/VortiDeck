/**
 * VortiDeck PDK Generator
 *
 * Multi-language PDK generation for VortiDeck plugins.
 * Extracts host function metadata from Rust code and generates
 * type-safe wrappers in various languages.
 *
 * Supported languages:
 * - TypeScript (for Extism JS plugins)
 * - Python (future)
 * - Go (future)
 */
pub mod pdk_gen;

// Re-export main types for convenience
pub use pdk_gen::{
    extract_host_functions, HostFunction, Parameter, PdkGenerator, TypeScriptGenerator,
};
