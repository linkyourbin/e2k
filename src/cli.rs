use clap::Parser;
use std::path::PathBuf;
use crate::error::{AppError, Result};

#[derive(Parser, Debug)]
#[command(name = "e2k")]
#[command(version = "0.1.0")]
#[command(about = "Convert EasyEDA/LCSC components to KiCad library formats", long_about = None)]
pub struct Cli {
    /// LCSC component ID (e.g., C2040)
    #[arg(long, value_name = "ID")]
    pub lcsc_id: String,

    /// Convert symbol only
    #[arg(long)]
    pub symbol: bool,

    /// Convert footprint only
    #[arg(long)]
    pub footprint: bool,

    /// Convert 3D model only
    #[arg(long = "3d")]
    pub model_3d: bool,

    /// Convert all (symbol + footprint + 3D model)
    #[arg(long)]
    pub full: bool,

    /// Output directory path
    #[arg(short, long, default_value = ".")]
    pub output: PathBuf,

    /// Overwrite existing components
    #[arg(long)]
    pub overwrite: bool,

    /// Use KiCad v5 legacy format
    #[arg(long)]
    pub v5: bool,

    /// Use project-relative paths for 3D models
    #[arg(long)]
    pub project_relative: bool,

    /// Enable debug logging
    #[arg(long)]
    pub debug: bool,
}

impl Cli {
    pub fn validate(&self) -> Result<()> {
        // Validate LCSC ID format (should start with C followed by digits)
        if !self.lcsc_id.starts_with('C') || self.lcsc_id.len() < 2 {
            return Err(AppError::Easyeda(
                crate::error::EasyedaError::InvalidLcscId(self.lcsc_id.clone())
            ));
        }

        // Check if at least one conversion option is selected
        if !self.symbol && !self.footprint && !self.model_3d && !self.full {
            return Err(AppError::Other(
                "At least one conversion option must be specified (--symbol, --footprint, --3d, or --full)".to_string()
            ));
        }

        Ok(())
    }

    pub fn kicad_version(&self) -> KicadVersion {
        if self.v5 {
            KicadVersion::V5
        } else {
            KicadVersion::V6
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadVersion {
    V5,
    V6,
}
