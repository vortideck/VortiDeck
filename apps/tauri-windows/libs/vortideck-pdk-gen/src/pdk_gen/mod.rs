/**
 * PDK Generation Module
 *
 * Language-agnostic PDK generation for VortiDeck plugins.
 * Extracts host function metadata and generates type-safe wrappers.
 */
pub mod schema;
pub mod typescript;

pub use schema::{extract_host_functions, HostFunction, Parameter};
pub use typescript::TypeScriptGenerator;

/// Trait for language-specific PDK generators
pub trait PdkGenerator {
    /// Generate complete PDK file for the target language
    fn generate(&self, functions: &[HostFunction]) -> String;

    /// Generate type declarations file for host functions (optional)
    fn generate_type_declarations(&self, _functions: &[HostFunction]) -> Option<String> {
        None
    }

    /// Generate SDK wrapper with clean API (api.ts for TypeScript)
    fn generate_sdk_wrapper(&self, _functions: &[HostFunction]) -> Option<String> {
        None
    }

    /// File extension for generated files
    fn file_extension(&self) -> &str;

    /// Language name for logging
    fn language_name(&self) -> &str;
}
