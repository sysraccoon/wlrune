mod config;
mod recognizer;
mod wayland;

use std::{
    collections::HashSet,
    env,
    fs::{create_dir_all, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use clap::Parser;
use config::AppConfig;
use recognizer::{degrees_to_radians, Point, Unistroke, UnistrokeRecognizer};
use wayland::get_user_gesture;

/// Mouse gestures for wayland compositors
#[derive(Parser, Debug)]
#[clap(author = "sysraccoon", version, about)]
struct AppArguments {
    #[clap(subcommand)]
    subcommand: AppSubCommand,
    /// Specify custom configuration file. By default used ~/.config/wlrune/config.yaml
    #[arg(long = "config", short = 'c')]
    config_path: Option<String>,
}

#[derive(Parser, Debug)]
enum AppSubCommand {
    /// Recognize pattern and execute related command
    Recognize,
    /// Record pattern for recognition. By default saved to ~/.local/share/wlrune/patterns
    Record(RecordArguments),
}

#[derive(Parser, Debug)]
struct RecordArguments {
    #[arg(long = "name", short = 'n')]
    name: String,
    #[arg(long = "force", short = 'f', default_value_t = false)]
    force: bool,
}

fn main() -> Result<(), ()> {
    let args = AppArguments::parse();
    let config = load_config(args.config_path.as_deref())?;

    match args.subcommand {
        AppSubCommand::Recognize => {
            if config.commands.len() == 0 {
                let config_path = args.config_path.unwrap_or_else(|| {
                    default_config_pathes().unwrap()[0].display().to_string()
                });
                eprintln!("command list is empty");
                eprintln!("record some patterns by using `wlrune record --name up`");
                eprintln!("and define commands in configuration file {}", &config_path);
                eprintln!("");
                eprintln!("# EXAMPLE CONFIGURATION #");
                eprintln!("commands:");
                eprintln!("  - pattern: \"up\"");
                eprintln!("    command: \"firefox\"");
                eprintln!("  - pattern: \"down\"");
                eprintln!("    command: \"kitty\"");
                eprintln!("#########################");
                return Err(());
            }

            let recognizer_conf = &config.recognizer;
            let pattern_names = config
                .commands
                .iter()
                .map(|cmd| cmd.pattern.as_str())
                .collect::<HashSet<_>>()
                .into_iter();
            let patterns = load_gestures(pattern_names).unwrap();

            let Some(gesture_path) = get_user_gesture() else {
                return Ok(());
            };

            if gesture_path.len() < config.recognizer.point_count_treshold as usize {
                eprintln!("skip gesture saving, reason: pattern point count less than specified in config ({})", config.recognizer.point_count_treshold);
                return Err(());
            }

            let mut unistroke_recognizer = UnistrokeRecognizer {
                angle_range_rad: degrees_to_radians(recognizer_conf.rotation_angle_range),
                angle_precision: degrees_to_radians(recognizer_conf.rotation_angle_treshold),
                width: recognizer_conf.width,
                height: recognizer_conf.height,
                resample_num_points: recognizer_conf.resample_num_points,
                patterns: Vec::new(),
            };

            for unistroke in &patterns {
                unistroke_recognizer.add_pattern(unistroke.name.clone(), &unistroke.path);
            }

            let (unistroke, similarity) = unistroke_recognizer.recognize_unistroke(&gesture_path);
            eprintln!(
                "recognized as {} (similarity ≈ {:.02})",
                unistroke.name, similarity
            );

            if similarity >= recognizer_conf.command_execute_treshold {
                let raw_command = &config
                    .commands
                    .iter()
                    .find(|cmd| cmd.pattern == unistroke.name)
                    .unwrap()
                    .command;

                let mut cmd = Command::new("bash");

                cmd.stderr(Stdio::null());
                cmd.stdout(Stdio::null());
                cmd.stdin(Stdio::null());

                cmd.arg("-c");
                cmd.arg(raw_command);

                let _child = cmd.spawn().unwrap();
            } else {
                eprintln!(
                    "skip command execution, reason: similarity less than specified in config ({})",
                    recognizer_conf.command_execute_treshold
                );
            }
        }
        AppSubCommand::Record(args) => {
            let gesture_file_path = gesture_file_path(&args.name)?;

            if gesture_file_path.exists() && !args.force {
                eprintln!("ERROR: pattern with name {} already exist, use --force flag if you want override it", &args.name);
                return Err(());
            }

            let Some(gesture_path) = get_user_gesture() else {
                return Ok(());
            };

            if gesture_path.len() < config.recognizer.point_count_treshold as usize {
                eprintln!("skip gesture saving, reason: pattern point count less than specified in config ({})", config.recognizer.point_count_treshold);
                return Err(());
            }

            save_gesture(&gesture_file_path, &gesture_path).unwrap();
        }
    }

    Ok(())
}

fn load_config(config_path: Option<&str>) -> Result<AppConfig, ()> 
{
    if let Some(config_path) = config_path {
        let config_path = Path::new(&config_path);
        return Ok(AppConfig::load(&config_path)?);
    }

    for config_path in default_config_pathes()? {
        if config_path.exists() {
            let config = AppConfig::load(&config_path)?;
            return Ok(config)
        }
    }

    Ok(AppConfig::default())
}

fn default_config_pathes() -> Result<Vec<PathBuf>, ()> {
    let home = env::var("HOME").map_err(|err| {
        eprintln!("ERROR: coludn't find $HOME: {err}");
    })?;

    let xdg_config_home = env::var("XDG_CONFIG_HOME").unwrap_or(format!("{home}/.local/share"));
    let xdg_config_home = Path::new(&xdg_config_home);

    Ok(vec![
      xdg_config_home.join("wlrune/config.yaml"),
      xdg_config_home.join("wlrune/config.yml"),
      xdg_config_home.join("wlrune/config"),
    ])
}

fn load_gestures<'a, I>(names: I) -> Result<Vec<Unistroke>, ()>
where
    I: Iterator<Item = &'a str>,
{
    let mut patterns = Vec::new();
    for name in names {
        let path = gesture_file_path(name)?;
        if path.is_file() || path.is_symlink() {
            let gesture_file = File::open(&path).map_err(|err| {
                eprintln!(
                    "ERROR: couldn't read gesture file {}, {}",
                    &path.display(),
                    err
                );
            })?;

            let mut pattern_path = Vec::new();
            let reader = BufReader::new(gesture_file);
            for line in reader.lines() {
                let line = line.map_err(|err| {
                    eprintln!("ERROR: couldn't read line {err}");
                })?;

                let items: Vec<&str> = line.split(" ").collect();
                let x: f64 = items[0].parse().unwrap();
                let y: f64 = items[1].parse().unwrap();

                pattern_path.push(Point::new(x, y));
            }

            patterns.push(Unistroke {
                name: name.to_string(),
                path: pattern_path,
            });
        }
    }

    Ok(patterns)
}

fn save_gesture(gesture_file_path: &Path, path: &[Point]) -> Result<(), ()> {
    let pattern_directory = gesture_file_path.parent().unwrap();
    create_dir_all(&pattern_directory).map_err(|err| {
        eprintln!(
            "ERROR: couldn't create {}: {}",
            pattern_directory.display(),
            err
        );
    })?;

    let mut gesture_file = File::create(&gesture_file_path).map_err(|err| {
        eprintln!("ERROR: couldn't open {}: {}", gesture_file_path.display(), err);
    })?;

    let serialized_path = path
        .iter()
        .map(|p| format!("{} {}", p.x, p.y))
        .collect::<Vec<_>>()
        .join("\n");

    gesture_file
        .write_all(serialized_path.as_bytes())
        .map_err(|err| {
            eprintln!("ERROR: cloudn't write gesture to file: {err}");
        })?;

    gesture_file.sync_all().map_err(|err| {
        eprintln!("ERROR: couldn't sync changes with file: {err}");
    })?;

    Ok(())
}

fn gesture_file_path(name: &str) -> Result<PathBuf, ()> {
    let gesture_data_dir = gesture_data_dir()?;
    let gesture_path = gesture_data_dir.join("patterns").join(name);

    Ok(gesture_path)
}

fn gesture_data_dir() -> Result<PathBuf, ()> {
    let home = env::var("HOME").map_err(|err| {
        eprintln!("ERROR: coludn't find $HOME: {err}");
    })?;
    let xdg_home = env::var("XDG_DATA_HOME").unwrap_or(format!("{home}/.local/share"));
    let xdg_home = Path::new(&xdg_home);
    let gesture_data_dir = xdg_home.join("wlrune");

    Ok(gesture_data_dir)
}
