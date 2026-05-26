use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const CORE_SEARCH_DIRS: &[&str] = &[
    "$HOME/.config/retroarch/cores",
    "$HOME/.var/app/org.libretro.RetroArch/config/retroarch/cores",
    "/usr/lib/libretro",
    "/usr/lib/x86_64-linux-gnu/libretro",
    "/usr/share/libretro/cores",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct System {
    id: &'static str,
    name: &'static str,
    core_name: &'static str,
    core_file: &'static str,
    extensions: &'static [&'static str],
}

const SYSTEMS: &[System] = &[
    System {
        id: "nes",
        name: "Nintendo Entertainment System",
        core_name: "Mesen",
        core_file: "mesen_libretro.so",
        extensions: &["nes", "fds"],
    },
    System {
        id: "snes",
        name: "Super Nintendo Entertainment System",
        core_name: "Snes9x - Current",
        core_file: "snes9x_libretro.so",
        extensions: &["sfc", "smc", "fig", "swc"],
    },
    System {
        id: "gb",
        name: "Game Boy",
        core_name: "SameBoy",
        core_file: "sameboy_libretro.so",
        extensions: &["gb"],
    },
    System {
        id: "gbc",
        name: "Game Boy Color",
        core_name: "SameBoy",
        core_file: "sameboy_libretro.so",
        extensions: &["gbc"],
    },
    System {
        id: "gba",
        name: "Game Boy Advance",
        core_name: "mGBA",
        core_file: "mgba_libretro.so",
        extensions: &["gba"],
    },
    System {
        id: "genesis",
        name: "Mega Drive / Genesis",
        core_name: "Genesis Plus GX",
        core_file: "genesis_plus_gx_libretro.so",
        extensions: &["md", "gen", "smd", "bin"],
    },
    System {
        id: "mastersystem",
        name: "Master System",
        core_name: "Genesis Plus GX",
        core_file: "genesis_plus_gx_libretro.so",
        extensions: &["sms"],
    },
    System {
        id: "gamegear",
        name: "Game Gear",
        core_name: "Genesis Plus GX",
        core_file: "genesis_plus_gx_libretro.so",
        extensions: &["gg"],
    },
    System {
        id: "32x",
        name: "Mega Drive / Genesis 32X",
        core_name: "PicoDrive",
        core_file: "picodrive_libretro.so",
        extensions: &["32x"],
    },
    System {
        id: "ps1",
        name: "PlayStation",
        core_name: "SwanStation",
        core_file: "swanstation_libretro.so",
        extensions: &["cue", "m3u", "pbp"],
    },
    System {
        id: "psp",
        name: "PlayStation Portable",
        core_name: "PPSSPP",
        core_file: "ppsspp_libretro.so",
        extensions: &["iso", "cso"],
    },
    System {
        id: "dreamcast",
        name: "Dreamcast",
        core_name: "Flycast",
        core_file: "flycast_libretro.so",
        extensions: &["gdi", "cdi"],
    },
    System {
        id: "n64",
        name: "Nintendo 64",
        core_name: "Mupen64Plus-Next",
        core_file: "mupen64plus_next_libretro.so",
        extensions: &["n64", "z64", "v64"],
    },
    System {
        id: "saturn",
        name: "Sega Saturn",
        core_name: "Beetle Saturn",
        core_file: "mednafen_saturn_libretro.so",
        extensions: &["cue", "m3u"],
    },
    System {
        id: "saturn-yabasanshiro",
        name: "Sega Saturn",
        core_name: "YabaSanshiro",
        core_file: "yabasanshiro_libretro.so",
        extensions: &[],
    },
    System {
        id: "fbneo",
        name: "FinalBurn Neo",
        core_name: "FinalBurn Neo",
        core_file: "fbneo_libretro.so",
        extensions: &["zip"],
    },
    System {
        id: "mame",
        name: "MAME Current",
        core_name: "MAME Current",
        core_file: "mame_libretro.so",
        extensions: &[],
    },
    System {
        id: "dos",
        name: "DOS",
        core_name: "DOSBox Pure",
        core_file: "dosbox_pure_libretro.so",
        extensions: &["dosz", "conf", "exe", "bat", "com"],
    },
    System {
        id: "scummvm",
        name: "ScummVM",
        core_name: "ScummVM",
        core_file: "scummvm_libretro.so",
        extensions: &["scummvm"],
    },
    System {
        id: "ds",
        name: "Nintendo DS",
        core_name: "melonDS",
        core_file: "melonds_libretro.so",
        extensions: &["nds"],
    },
    System {
        id: "3ds",
        name: "Nintendo 3DS",
        core_name: "Citra",
        core_file: "citra_libretro.so",
        extensions: &["3ds", "cci", "cxi"],
    },
];

const AMBIGUOUS_EXTENSIONS: &[&str] = &["7z", "bin", "chd", "cue", "iso", "m3u", "zip"];

#[derive(Debug, Default)]
struct NoxGameManifest {
    name: Option<String>,
    system: Option<String>,
    rom: Option<PathBuf>,
    core: Option<String>,
    fullscreen: Option<bool>,
}

#[derive(Debug)]
enum NoxError {
    Io(io::Error),
    InvalidManifest(String),
    UnknownSystem(String),
    UnsupportedRom(PathBuf),
    AmbiguousRom(PathBuf, String),
    MissingCore { core_file: String },
    MissingRom(PathBuf),
    Launch(io::Error),
}

impl fmt::Display for NoxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::InvalidManifest(message) => {
                write!(formatter, "invalid .noxgame manifest: {message}")
            }
            Self::UnknownSystem(system) => write!(formatter, "unknown game system `{system}`"),
            Self::UnsupportedRom(path) => write!(
                formatter,
                "could not detect a supported system for {}",
                path.display()
            ),
            Self::AmbiguousRom(path, extension) => write!(
                formatter,
                "{} uses ambiguous extension `.{extension}`; create a .noxgame manifest with an explicit `system` or `core`",
                path.display()
            ),
            Self::MissingCore { core_file } => write!(
                formatter,
                "RetroArch core `{core_file}` was not found in the known core directories"
            ),
            Self::MissingRom(path) => write!(formatter, "ROM does not exist: {}", path.display()),
            Self::Launch(error) => write!(formatter, "failed to launch RetroArch: {error}"),
        }
    }
}

impl From<io::Error> for NoxError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
struct LaunchPlan {
    name: String,
    system: Option<System>,
    rom_path: PathBuf,
    core_file: String,
    core_path: PathBuf,
    fullscreen: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("noxgames: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), NoxError> {
    let mut args = env::args_os();
    let _program = args.next();
    let Some(command) = args.next() else {
        print_usage();
        return Ok(());
    };

    match command.to_string_lossy().as_ref() {
        "run" => {
            let Some(target) = args.next() else {
                return Err(NoxError::InvalidManifest(
                    "usage: noxgames run <rom-or-manifest> [--dry-run]".to_string(),
                ));
            };
            let dry_run = args.any(|arg| arg == OsStr::new("--dry-run"));
            run_target(Path::new(&target), dry_run)
        }
        "detect" => {
            let Some(target) = args.next() else {
                return Err(NoxError::InvalidManifest(
                    "usage: noxgames detect <rom-or-manifest>".to_string(),
                ));
            };
            let plan = build_launch_plan(Path::new(&target))?;
            print_plan(&plan);
            Ok(())
        }
        "cores" => {
            print_core_report();
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(NoxError::InvalidManifest(format!(
            "unknown command `{other}`; run `noxgames help`"
        ))),
    }
}

fn run_target(target: &Path, dry_run: bool) -> Result<(), NoxError> {
    let plan = build_launch_plan(target)?;
    print_plan(&plan);

    if dry_run {
        return Ok(());
    }

    let mut command = Command::new("retroarch");
    command.arg("-L").arg(&plan.core_path).arg(&plan.rom_path);

    if plan.fullscreen {
        command.arg("--fullscreen");
    }

    command.spawn().map(|_| ()).map_err(NoxError::Launch)
}

fn build_launch_plan(target: &Path) -> Result<LaunchPlan, NoxError> {
    if is_noxgame_manifest(target) {
        let manifest = parse_manifest(target)?;
        let manifest_dir = target.parent().unwrap_or_else(|| Path::new("."));
        let rom_path = manifest
            .rom
            .as_deref()
            .map(|path| resolve_relative_path(manifest_dir, path))
            .ok_or_else(|| NoxError::InvalidManifest("missing top-level `rom`".to_string()))?;

        if !rom_path.exists() {
            return Err(NoxError::MissingRom(rom_path));
        }

        let system = match manifest.system.as_deref() {
            Some(system_id) => Some(system_by_id(system_id)?),
            None => detect_system(&rom_path)?,
        };
        let core_file = manifest.core.unwrap_or_else(|| {
            system
                .map(|detected| detected.core_file.to_string())
                .unwrap_or_default()
        });

        if core_file.is_empty() {
            return Err(NoxError::InvalidManifest(
                "manifest must define `core` when `system` cannot be detected".to_string(),
            ));
        }

        let core_path = resolve_core_path(&core_file)?;
        let name = manifest
            .name
            .unwrap_or_else(|| game_name_from_path(&rom_path));

        return Ok(LaunchPlan {
            name,
            system,
            rom_path,
            core_file,
            core_path,
            fullscreen: manifest.fullscreen.unwrap_or(false),
        });
    }

    if !target.exists() {
        return Err(NoxError::MissingRom(target.to_path_buf()));
    }

    let system =
        detect_system(target)?.ok_or_else(|| NoxError::UnsupportedRom(target.to_path_buf()))?;
    let core_file = system.core_file.to_string();
    let core_path = resolve_core_path(&core_file)?;

    Ok(LaunchPlan {
        name: game_name_from_path(target),
        system: Some(system),
        rom_path: target.to_path_buf(),
        core_file,
        core_path,
        fullscreen: false,
    })
}

fn parse_manifest(path: &Path) -> Result<NoxGameManifest, NoxError> {
    let manifest_text = fs::read_to_string(path)?;
    let mut manifest = NoxGameManifest::default();
    let mut section = String::new();

    for (line_number, raw_line) in manifest_text.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            section = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(NoxError::InvalidManifest(format!(
                "line {} is not a key/value pair",
                line_number + 1
            )));
        };
        let key = raw_key.trim();
        let value = parse_manifest_value(raw_value.trim());
        let qualified_key = if section.is_empty() {
            key.to_string()
        } else {
            format!("{section}.{key}")
        };

        match qualified_key.as_str() {
            "name" => manifest.name = Some(value),
            "system" => manifest.system = Some(value),
            "rom" => manifest.rom = Some(PathBuf::from(value)),
            "runner.core" => manifest.core = Some(value),
            "window.fullscreen" => manifest.fullscreen = Some(parse_bool(&value)?),
            _ => {}
        }
    }

    Ok(manifest)
}

fn parse_manifest_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        trimmed.to_string()
    }
}

fn parse_bool(value: &str) -> Result<bool, NoxError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(NoxError::InvalidManifest(format!(
            "`{other}` is not a valid boolean"
        ))),
    }
}

fn detect_system(path: &Path) -> Result<Option<System>, NoxError> {
    let extension = match extension_lowercase(path) {
        Some(extension) => extension,
        None => return Ok(None),
    };

    if AMBIGUOUS_EXTENSIONS.contains(&extension.as_str()) {
        return Err(NoxError::AmbiguousRom(path.to_path_buf(), extension));
    }

    Ok(SYSTEMS
        .iter()
        .copied()
        .find(|system| system.extensions.contains(&extension.as_str())))
}

fn system_by_id(system_id: &str) -> Result<System, NoxError> {
    let normalized = system_id.trim().to_lowercase();
    SYSTEMS
        .iter()
        .copied()
        .find(|system| system.id == normalized)
        .ok_or_else(|| NoxError::UnknownSystem(system_id.to_string()))
}

fn resolve_core_path(core_file: &str) -> Result<PathBuf, NoxError> {
    let core_path = PathBuf::from(core_file);
    if core_path.is_absolute() && core_path.exists() {
        return Ok(core_path);
    }

    for directory in core_search_dirs() {
        let candidate = directory.join(core_file);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(NoxError::MissingCore {
        core_file: core_file.to_string(),
    })
}

fn core_search_dirs() -> Vec<PathBuf> {
    CORE_SEARCH_DIRS
        .iter()
        .map(|path| expand_home(path))
        .collect()
}

fn expand_home(path: &str) -> PathBuf {
    if let Some(suffix) = path.strip_prefix("$HOME/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(suffix);
        }
    }
    PathBuf::from(path)
}

fn resolve_relative_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn is_noxgame_manifest(path: &Path) -> bool {
    extension_lowercase(path).is_some_and(|extension| extension == "noxgame")
}

fn extension_lowercase(path: &Path) -> Option<String> {
    path.extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
}

fn game_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(OsStr::to_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn print_plan(plan: &LaunchPlan) {
    println!("Game: {}", plan.name);
    if let Some(system) = plan.system {
        println!("System: {} ({})", system.name, system.id);
        println!("Core: {} ({})", system.core_name, plan.core_file);
    } else {
        println!("System: custom");
        println!("Core: {}", plan.core_file);
    }
    println!("Core path: {}", plan.core_path.display());
    println!("ROM: {}", plan.rom_path.display());
    println!("Fullscreen: {}", plan.fullscreen);
}

fn print_core_report() {
    println!("RetroArch core search paths:");
    for directory in core_search_dirs() {
        println!("  {}", directory.display());
    }
    println!();
    println!("Known systems:");
    for system in SYSTEMS {
        let status = match resolve_core_path(system.core_file) {
            Ok(path) => format!("found at {}", path.display()),
            Err(_) => "missing".to_string(),
        };
        println!(
            "  {:13} {:34} {:22} {}",
            system.id, system.name, system.core_name, status
        );
    }
}

fn print_usage() {
    println!("noxgames");
    println!();
    println!("Usage:");
    println!("  noxgames run <rom-or-manifest> [--dry-run]");
    println!("  noxgames detect <rom-or-manifest>");
    println!("  noxgames cores");
    println!();
    println!("Manifest example:");
    println!("  name = \"Example Game\"");
    println!("  system = \"gba\"");
    println!("  rom = \"Example Game.gba\"");
    println!();
    println!("  [runner]");
    println!("  core = \"mgba_libretro.so\"");
}
