use crate::error::{KicadError, Result};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

pub struct LibraryManager {
    output_path: PathBuf,
}

impl LibraryManager {
    pub fn new(output_path: &Path) -> Self {
        Self {
            output_path: output_path.to_path_buf(),
        }
    }

    /// Create necessary output directories
    pub fn create_directories(&self) -> Result<()> {
        // Create main output directory
        fs::create_dir_all(&self.output_path)
            .map_err(KicadError::Io)?;

        // Create .pretty directory for footprints
        let pretty_dir = self.output_path.join("e2k.pretty");
        fs::create_dir_all(&pretty_dir)
            .map_err(KicadError::Io)?;

        // Create .3dshapes directory for 3D models
        let shapes_dir = self.output_path.join("e2k.3dshapes");
        fs::create_dir_all(&shapes_dir)
            .map_err(KicadError::Io)?;

        Ok(())
    }

    /// Check if a component exists in the library file
    pub fn component_exists(&self, lib_path: &Path, component_name: &str) -> Result<bool> {
        if !lib_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(lib_path)
            .map_err(KicadError::Io)?;

        // Check for v6 format
        let v6_pattern = format!(r#"\(symbol\s+"{}""#, regex::escape(component_name));
        if let Ok(re) = Regex::new(&v6_pattern) {
            if re.is_match(&content) {
                return Ok(true);
            }
        }

        // Check for v5 format
        let v5_pattern = format!(r"DEF\s+{}\s+", regex::escape(component_name));
        if let Ok(re) = Regex::new(&v5_pattern) {
            if re.is_match(&content) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Add a component to the library file
    pub fn add_component(&self, lib_path: &Path, component_data: &str) -> Result<()> {
        let mut content = if lib_path.exists() {
            // Read existing file and remove the closing parenthesis
            let existing = fs::read_to_string(lib_path)
                .map_err(KicadError::Io)?;
            // Remove trailing ')' and whitespace
            existing.trim_end().trim_end_matches(')').to_string()
        } else {
            // Create new library file with header (v6 format with proper formatting)
            if component_data.contains("(symbol") {
                // v6 format - match Python's formatting exactly
                String::from("(kicad_symbol_lib\n  (version 20211014)\n  (generator e2k)")
            } else {
                // v5 format
                String::from("EESchema-LIBRARY Version 2.4\n#encoding utf-8")
            }
        };

        // Append component
        content.push('\n');
        content.push_str(component_data);

        // Add closing parenthesis for v6 format
        if component_data.contains("(symbol") {
            content.push('\n');
            content.push(')');
        }
        content.push('\n');

        fs::write(lib_path, content)
            .map_err(KicadError::Io)?;

        Ok(())
    }

    /// Update an existing component in the library file
    pub fn update_component(&self, lib_path: &Path, component_name: &str, new_data: &str) -> Result<()> {
        let content = fs::read_to_string(lib_path)
            .map_err(KicadError::Io)?;

        // Try v6 format first
        let v6_pattern = format!(
            r#"\(symbol\s+"{}"\s+.*?\n\)\n"#,
            regex::escape(component_name)
        );
        if let Ok(re) = Regex::new(&v6_pattern) {
            if re.is_match(&content) {
                let new_content = re.replace(&content, new_data);
                fs::write(lib_path, new_content.as_ref())
                    .map_err(KicadError::Io)?;
                return Ok(());
            }
        }

        // Try v5 format
        let v5_pattern = format!(
            r"DEF\s+{}\s+.*?ENDDEF\n",
            regex::escape(component_name)
        );
        if let Ok(re) = Regex::new(&v5_pattern) {
            if re.is_match(&content) {
                let new_content = re.replace(&content, new_data);
                fs::write(lib_path, new_content.as_ref())
                    .map_err(KicadError::Io)?;
                return Ok(());
            }
        }

        Err(KicadError::SymbolExport(format!("Component {} not found in library", component_name)).into())
    }

    /// Write a footprint file
    pub fn write_footprint(&self, footprint_name: &str, data: &str) -> Result<PathBuf> {
        let pretty_dir = self.output_path.join("e2k.pretty");
        let footprint_path = pretty_dir.join(format!("{}.kicad_mod", footprint_name));

        fs::write(&footprint_path, data)
            .map_err(KicadError::Io)?;

        log::info!("Wrote footprint: {}", footprint_path.display());

        Ok(footprint_path)
    }

    /// Write 3D model files
    pub fn write_3d_model(&self, model_name: &str, wrl_data: &str, step_data: &[u8]) -> Result<(PathBuf, PathBuf)> {
        let shapes_dir = self.output_path.join("e2k.3dshapes");

        // Write VRML file
        let wrl_path = shapes_dir.join(format!("{}.wrl", model_name));
        fs::write(&wrl_path, wrl_data)
            .map_err(KicadError::Io)?;

        log::info!("Wrote VRML model: {}", wrl_path.display());

        // Write STEP file
        let step_path = shapes_dir.join(format!("{}.step", model_name));
        fs::write(&step_path, step_data)
            .map_err(KicadError::Io)?;

        log::info!("Wrote STEP model: {}", step_path.display());

        Ok((wrl_path, step_path))
    }

    /// Get the symbol library path
    pub fn get_symbol_lib_path(&self, v5: bool) -> PathBuf {
        if v5 {
            self.output_path.join("e2k.lib")
        } else {
            self.output_path.join("e2k.kicad_sym")
        }
    }
}
