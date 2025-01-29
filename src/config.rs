use std::{fs::File, io::Read, path::Path};

use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Configuration related to pattern recognition
    #[serde(default)]
    pub recognizer: RecognizerConfig,
    /// Patterns that associated with commands (Pattern => Command)
    #[serde(default)]
    pub commands: Vec<GestureCommand>,
}

#[serde_inline_default]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognizerConfig {
    /// The percentage of similarity between the original pattern
    /// and the user input requiret to trigger the command
    #[serde_inline_default(0.8)]
    pub command_execute_treshold: f64,
    /// Point count required to trigger command or save new pattern
    #[serde_inline_default(10)]
    pub point_count_treshold: u64,
    /// Acceptable range for pattern rotation (degrees)
    #[serde_inline_default(10.0)]
    pub rotation_angle_range: f64,
    /// Acceptable accuracy in pattern rotation (degrees)
    #[serde_inline_default(2.0)]
    pub rotation_angle_treshold: f64,
    /// The number of points to which the pattern is reduced fo recognition
    #[serde_inline_default(64)]
    pub resample_num_points: u32,
    /// Width used for recognition (may not match screen size)
    #[serde_inline_default(100.0)]
    pub width: f64,
    /// Height used for recognition (may not match screen size)
    #[serde_inline_default(100.0)]
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureCommand {
    pub pattern: String,
    pub command: String,
}

impl AppConfig {
    pub fn load(config_path: &Path) -> Result<Self, ()> {
        let mut file = File::open(config_path).map_err(|err| {
            eprintln!("ERROR: failed to open config {}, {}", config_path.display(), err);
        })?;

        let mut raw = String::new();
        file.read_to_string(&mut raw).map_err(|err| {
            eprintln!("ERROR: failed to read from config {}, {}", config_path.display(), err);
        })?;

        let config: AppConfig = serde_yml::from_str(&raw).map_err(|err| {
            eprintln!("ERROR: failed to parse config {}, {}", config_path.display(), err);
        })?;

        let exec_treshold = config.recognizer.command_execute_treshold;
        if exec_treshold < 0.0 || exec_treshold > 1.0 {
            eprintln!("ERROR: recognizer.command_execute_treshold should be in range [0,1]");
            return Err(());
        }

        if config.recognizer.width <= 0.0 {
            eprintln!("ERROR: recognizer.width should be positive number");
            return Err(());
        }

        if config.recognizer.height <= 0.0 {
            eprintln!("ERROR: recognizer.height should be positive number");
            return Err(());
        }

        Ok(config)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        serde_yml::from_str("").unwrap()
    }
}

impl Default for RecognizerConfig {
    fn default() -> Self {
        serde_yml::from_str("").unwrap()
    }
}

