//! Plugin file validation module
//!
//! Provides validation for plugin files without code duplication.
//! Validates file formats by checking magic numbers efficiently.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Magic number constants
const ZIP_MAGIC: &[u8; 2] = b"PK"; // ZIP files (used by .vdext bundles)
const WASM_MAGIC: &[u8; 4] = b"\0asm"; // WebAssembly modules

/// Plugin file type detected by extension and validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginFileType {
    /// .vdext bundle (ZIP archive with plugin.wasm + web extensions)
    VdextBundle,
    /// Legacy .wasm file (standalone WebAssembly module)
    WasmModule,
    /// Unknown or unsupported file type
    Unknown,
}

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validation errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid .vdext bundle: not a ZIP file")]
    InvalidVdextFormat,

    #[error("Invalid .wasm file: not a WebAssembly module")]
    InvalidWasmFormat,

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    #[error("File too small to validate: {0} bytes")]
    FileTooSmall(usize),
}

impl PluginFileType {
    /// Detect plugin file type from extension
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|s| s.to_str()) {
            Some("vdext") => Self::VdextBundle,
            Some("wasm") => Self::WasmModule,
            _ => Self::Unknown,
        }
    }

    /// Check if this is a known plugin type
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::VdextBundle | Self::WasmModule)
    }

    /// Get file extension
    pub fn extension(&self) -> &'static str {
        match self {
            Self::VdextBundle => "vdext",
            Self::WasmModule => "wasm",
            Self::Unknown => "unknown",
        }
    }
}

/// Validate a plugin file by checking its magic number
///
/// This function efficiently reads only the necessary bytes to validate
/// the file format without loading the entire file into memory.
pub fn validate_plugin_file(path: &Path) -> ValidationResult<PluginFileType> {
    let file_type = PluginFileType::from_path(path);

    if !file_type.is_valid() {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        return Err(ValidationError::UnsupportedFileType(ext.to_string()));
    }

    match file_type {
        PluginFileType::VdextBundle => validate_zip_format(path)?,
        PluginFileType::WasmModule => validate_wasm_format(path)?,
        PluginFileType::Unknown => unreachable!(), // Already checked above
    }

    Ok(file_type)
}

/// Validate ZIP format by checking magic number (PK header)
///
/// Only reads the first 2 bytes for efficiency
fn validate_zip_format(path: &Path) -> ValidationResult<()> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 2];

    file.read_exact(&mut magic)
        .map_err(|_| ValidationError::FileTooSmall(0))?;

    if &magic != ZIP_MAGIC {
        return Err(ValidationError::InvalidVdextFormat);
    }

    Ok(())
}

/// Validate WASM format by checking magic number (\0asm header)
///
/// Only reads the first 4 bytes for efficiency
fn validate_wasm_format(path: &Path) -> ValidationResult<()> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 4];

    file.read_exact(&mut magic)
        .map_err(|_| ValidationError::FileTooSmall(0))?;

    if &magic != WASM_MAGIC {
        return Err(ValidationError::InvalidWasmFormat);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_type_detection() {
        assert_eq!(
            PluginFileType::from_path(Path::new("plugin.vdext")),
            PluginFileType::VdextBundle
        );
        assert_eq!(
            PluginFileType::from_path(Path::new("plugin.wasm")),
            PluginFileType::WasmModule
        );
        assert_eq!(
            PluginFileType::from_path(Path::new("plugin.txt")),
            PluginFileType::Unknown
        );
    }

    #[test]
    fn test_validate_zip() -> std::io::Result<()> {
        let mut file = NamedTempFile::new()?;
        file.write_all(b"PK\x03\x04")?; // Valid ZIP header

        assert!(validate_zip_format(file.path()).is_ok());
        Ok(())
    }

    #[test]
    fn test_validate_wasm() -> std::io::Result<()> {
        let mut file = NamedTempFile::new()?;
        file.write_all(b"\0asm\x01\x00\x00\x00")?; // Valid WASM header

        assert!(validate_wasm_format(file.path()).is_ok());
        Ok(())
    }
}
