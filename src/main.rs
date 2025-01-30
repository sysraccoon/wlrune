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

#[derive(Parser, Debug)]
#[clap(author = "sysraccoon", version, about)]
struct AppArguments {
    #[clap(subcommand)]
    subcommand: AppSubCommand,
    #[arg(
        long = "config",
        short = 'c',
    )]
    config_path: Option<String>,
}

#[derive(Parser, Debug)]
enum AppSubCommand {
    Recognize,
    Record(RecordArguments),
}

#[derive(Parser, Debug)]
struct RecordArguments {
    #[arg(long = "name", short = 'n')]
    name: String,
}

fn main() -> Result<(), ()> {
    let args = AppArguments::parse();
    let config = load_config(args.config_path)?;

    let Some(gesture_path) = get_user_gesture() else {
        return Ok(());
    };

    if gesture_path.len() < config.recognizer.point_count_treshold as usize {
        eprintln!("too few points");
        return Err(());
    }

    match args.subcommand {
        AppSubCommand::Recognize => {
            let recognizer_conf = config.recognizer;
            let pattern_names = config
                .commands
                .iter()
                .map(|cmd| cmd.pattern.as_str())
                .collect::<HashSet<_>>()
                .into_iter();
            let patterns = load_gestures(pattern_names).unwrap();
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
            eprintln!("recognized as {} ({})", unistroke.name, similarity);

            if similarity > recognizer_conf.command_execute_treshold {
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
            }
        }
        AppSubCommand::Record(args) => {
            record_gesture(&args.name, &gesture_path).unwrap();
        }
    }

    Ok(())
}

fn load_config(config_path: Option<String>,) -> Result<AppConfig, ()> {
    if let Some(config_path) = config_path {
        let config_path = Path::new(&config_path);
        return Ok(AppConfig::load(&config_path)?);
    }

    let home = env::var("HOME").map_err(|err| {
        eprintln!("ERROR: coludn't find $HOME: {err}");
    })?;

    let xdg_config_home = env::var("XDG_CONFIG_HOME").unwrap_or(format!("{home}/.local/share"));
    let xdg_config_home = Path::new(&xdg_config_home);
    let config_path = xdg_config_home.join("wlrune/config.yaml");

    let config = if config_path.exists() {
        eprintln!("load configuration file {}", config_path.display());
        AppConfig::load(&config_path)?
    } else {
        eprintln!("config {} not found, load default configuration", config_path.display());
        AppConfig::default()
    };

    Ok(config)
}

fn load_gestures<'a, I>(names: I) -> Result<Vec<Unistroke>, ()>
where
    I: Iterator<Item = &'a str>,
{
    let gesture_data_dir = gesture_data_dir()?;
    let mut patterns = Vec::new();

    for name in names {
        let path = gesture_data_dir.join(Path::new(&name));
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

fn record_gesture(name: &str, path: &[Point]) -> Result<(), ()> {
    let gesture_data_dir = gesture_data_dir()?;
    create_dir_all(&gesture_data_dir).map_err(|err| {
        eprintln!(
            "ERROR: couldn't create {}: {}",
            gesture_data_dir.display(),
            err
        );
    })?;

    let gesture_path = gesture_data_dir.join(name);

    let mut gesture_file = File::create(&gesture_path).map_err(|err| {
        eprintln!("ERROR: couldn't open {}: {}", gesture_path.display(), err);
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

fn gesture_data_dir() -> Result<PathBuf, ()> {
    let home = env::var("HOME").map_err(|err| {
        eprintln!("ERROR: coludn't find $HOME: {err}");
    })?;
    let xdg_home = env::var("XDG_DATA_HOME").unwrap_or(format!("{home}/.local/share"));
    let xdg_home = Path::new(&xdg_home);
    let gesture_data_dir = xdg_home.join("wlrune");

    Ok(gesture_data_dir)
}
