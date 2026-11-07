use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
/// Workspace Manager for VortiDeck
///
/// Manages the workspace directory structure in Documents/VortiDeck/workspace/
/// Provides secure path validation to prevent directory traversal attacks
/// Supports symlinks for advanced users who want to use external drives
use std::path::{Path, PathBuf};

/// Workspace directory structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Root workspace path: Documents/VortiDeck/workspace/
    pub root: PathBuf,
    /// Output directory for flow execution results
    pub output: PathBuf,
    /// Input directory for user files
    pub input: PathBuf,
    /// Temporary directory for processing
    pub temp: PathBuf,
}

impl WorkspaceConfig {
    /// Initialize workspace with given documents directory
    ///
    /// # Arguments
    /// * `documents_dir` - Path to user's Documents folder (from Tauri's document_dir())
    ///
    /// # Returns
    /// * `Result<WorkspaceConfig>` - Initialized workspace configuration
    pub fn new(documents_dir: PathBuf) -> Result<Self> {
        let root = documents_dir.join("VortiDeck").join("workspace");
        let output = root.join("output");
        let input = root.join("input");
        let temp = root.join("temp");

        let config = Self {
            root,
            output,
            input,
            temp,
        };

        // Create directories if they don't exist
        config.ensure_directories()?;

        Ok(config)
    }

    /// Ensure all workspace directories exist
    fn ensure_directories(&self) -> Result<()> {
        let root_existed = self.root.exists();

        fs::create_dir_all(&self.root).context("Failed to create workspace root directory")?;

        fs::create_dir_all(&self.output).context("Failed to create output directory")?;

        fs::create_dir_all(&self.input).context("Failed to create input directory")?;

        fs::create_dir_all(&self.temp).context("Failed to create temp directory")?;

        // Only log if we actually created the workspace
        if !root_existed {
            log::info!("Created workspace directories at: {}", self.root.display());
        }

        Ok(())
    }

    /// Validate that a file path is within the workspace
    ///
    /// This prevents directory traversal attacks (../../../etc/passwd)
    /// Resolves symlinks to check the actual location
    ///
    /// # Arguments
    /// * `path` - Path to validate
    ///
    /// # Returns
    /// * `Result<PathBuf>` - Canonicalized path if valid
    /// * `Err` - If path is outside workspace
    pub fn validate_path(&self, path: &Path) -> Result<PathBuf> {
        // Resolve symlinks and get canonical path
        let canonical_path = fs::canonicalize(path).context("Failed to resolve path")?;

        let canonical_workspace =
            fs::canonicalize(&self.root).context("Failed to resolve workspace root")?;

        // Check if path is within workspace
        if !canonical_path.starts_with(&canonical_workspace) {
            bail!(
                "Security error: Path '{}' is outside workspace '{}'. \
                Only files within the workspace are accessible.",
                canonical_path.display(),
                canonical_workspace.display()
            );
        }

        Ok(canonical_path)
    }

    /// Validate path without requiring it to exist (for writing new files)
    ///
    /// # Arguments
    /// * `path` - Path to validate
    ///
    /// # Returns
    /// * `Result<PathBuf>` - Validated path if within workspace
    pub fn validate_path_for_write(&self, path: &Path) -> Result<PathBuf> {
        // Convert to absolute path
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        // Check for directory traversal attempts
        let path_str = absolute_path.to_string_lossy();
        if path_str.contains("..") {
            bail!("Security error: Path contains '..' which could be a directory traversal attack");
        }

        // Verify it starts with workspace root
        if !absolute_path.starts_with(&self.root) {
            bail!(
                "Security error: Path '{}' is outside workspace '{}'. \
                Only files within the workspace can be written.",
                absolute_path.display(),
                self.root.display()
            );
        }

        Ok(absolute_path)
    }

    /// Get the output directory path
    pub fn get_output_dir(&self) -> &Path {
        &self.output
    }

    /// Get the input directory path
    pub fn get_input_dir(&self) -> &Path {
        &self.input
    }

    /// Get the temp directory path
    pub fn get_temp_dir(&self) -> &Path {
        &self.temp
    }

    /// Get the workspace root path
    pub fn get_root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_workspace_initialization() {
        let temp_dir = env::temp_dir().join("vortideck_test");
        let workspace = WorkspaceConfig::new(temp_dir.clone()).unwrap();

        assert!(workspace.root.exists());
        assert!(workspace.output.exists());
        assert!(workspace.input.exists());
        assert!(workspace.temp.exists());

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_path_validation() {
        let temp_dir = env::temp_dir().join("vortideck_test_validation");
        let workspace = WorkspaceConfig::new(temp_dir.clone()).unwrap();

        // Create a test file within workspace
        let test_file = workspace.output.join("test.txt");
        fs::write(&test_file, "test").unwrap();

        // Should succeed
        assert!(workspace.validate_path(&test_file).is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_directory_traversal_prevention() {
        let temp_dir = env::temp_dir().join("vortideck_test_traversal");
        let workspace = WorkspaceConfig::new(temp_dir.clone()).unwrap();

        // Attempt directory traversal
        let malicious_path = workspace.root.join("../../../etc/passwd");

        // Should fail validation
        assert!(workspace.validate_path_for_write(&malicious_path).is_err());

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }
}
