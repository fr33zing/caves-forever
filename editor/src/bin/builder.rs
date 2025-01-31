use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::anyhow;
use bytesize::ByteSize;
use clap::{Parser, ValueEnum};
use tracing::{debug, error, info, span, warn, Level};
use tracing_subscriber::util::SubscriberInitExt;
use walkdir::WalkDir;

use editor_lib::{
    data::Environment,
    state::{EditorMode, FilePayload},
};
use lib::worldgen::asset::AssetCollection;

#[derive(Parser, Clone)]
#[command(name = "Asset Builder")]
#[command(about = "Builds assets into a format consumable by the main game.")]
struct Args {
    /// Which environment to build for.
    #[arg(value_enum, short, long, default_value = "production")]
    env: Environment,

    /// Directory that contains the editor output.
    #[arg(short, long, default_value = "./assets/worldgen")]
    input: PathBuf,

    /// Output directory.
    #[arg(short, long, default_value = "./assets")]
    output: PathBuf,

    /// Output file prefix.
    #[arg(short, long, default_value = "worldgen")]
    name: String,

    /// Output file format. Only CBOR is used in-game, any other format is for debugging.
    #[arg(short, long, default_value = "cbor")]
    format: Format,
}

#[derive(Clone, PartialEq, ValueEnum, strum::Display)]
enum Format {
    Cbor,
    Ron,
}

#[derive(Default)]
struct Statistics {
    skipped: u32,
    failed: u32,
    succeeded: u32,
}

#[derive(PartialEq, strum::Display)]
#[repr(u8)]
enum Code {
    Success = 0,
    BuildError = 1,
    NoOutput = 2,
    WriteError = 3,
}

fn exit_error(code: Code, error: Option<anyhow::Error>) -> ! {
    if code != Code::Success {
        if let Some(error) = error {
            error!(
                code = code.to_string(),
                error = error.to_string(),
                "exiting with error"
            );
        } else {
            error!(code = code.to_string(), "exiting with error");
        }
    }
    std::process::exit(code as i32)
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .compact()
        .without_time()
        .finish()
        .init();

    let args = Args::parse();

    let assets = match build(args.clone()) {
        Ok((stats, assets)) => {
            if !check_build_statistics(&stats) {
                exit_error(Code::NoOutput, None);
            }
            assets
        }
        Err(error) => {
            exit_error(Code::BuildError, Some(error));
        }
    };

    match write_archive(args, assets) {
        Ok((file, size)) => {
            info!(
                file = file.display().to_string(),
                size = ByteSize(size as u64).to_string_as(false),
                "write succeeded"
            );
        }
        Err(error) => {
            exit_error(Code::WriteError, Some(error));
        }
    };
}

fn build(Args { env, input, .. }: Args) -> anyhow::Result<(Statistics, AssetCollection)> {
    let stats = Arc::new(Mutex::new(Statistics::default()));

    let files = filter_input_files(input)?;
    let assets = build_asset_collection(stats.clone(), env, files)?;

    let stats = Arc::try_unwrap(stats)
        .map_err(|_| anyhow!("unwrapping statistics failed"))?
        .into_inner()?;

    Ok((stats, assets))
}

fn check_build_statistics(stats: &Statistics) -> bool {
    let mut message = "build".to_string();
    if stats.succeeded > 0 {
        message += " succeeded";

        if stats.failed > 0 {
            message += " with failures";
        }
    } else {
        message += " failed (no output)";
        error!(
            message,
            skipped = stats.skipped,
            failed = stats.failed,
            succeeded = stats.succeeded,
        );
        return false;
    }

    info!(
        message,
        skipped = stats.skipped,
        failed = stats.failed,
        succeeded = stats.succeeded,
    );
    return true;
}

fn write_archive(
    Args {
        env,
        output,
        name,
        format,
        ..
    }: Args,
    assets: AssetCollection,
) -> anyhow::Result<(PathBuf, u64)> {
    let file_name = format!(
        "{name}.{}.{}",
        env.to_string().to_lowercase(),
        format.to_string().to_lowercase()
    );
    let path = output.join(file_name);
    let mut file = File::create(path.clone())?;

    let bytes = match format {
        Format::Cbor => cbor4ii::serde::to_vec(Vec::new(), &assets)?,
        Format::Ron => ron::ser::to_string_pretty(&assets, ron::ser::PrettyConfig::default())?
            .as_bytes()
            .to_vec(),
    };

    let size = bytes.len() as u64;
    file.write_all(&bytes)?;

    Ok((path, size))
}

fn filter_input_files(path: PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    let span = span!(Level::TRACE, "filter");
    let _enter = span.enter();

    let mut result = Vec::new();

    for entry in WalkDir::new(path) {
        let entry = entry?;
        let path = entry.path();

        let skip = |reason: &str| {
            debug!(path = path.display().to_string(), reason, "skip");
        };

        if path.is_dir() {
            skip("directory");
            continue;
        }
        let Some(file_name) = entry.file_name().to_str() else {
            skip("invalid filename");
            continue;
        };
        if file_name.starts_with(".") {
            skip("hidden");
            continue;
        }
        let Ok(mode) = EditorMode::from_path(path) else {
            skip("not an editor file");
            continue;
        };

        debug!(
            path = path.display().to_string(),
            mode = mode.to_string(),
            "keep"
        );
        result.push(path.to_owned());
    }

    Ok(result)
}

fn build_asset_collection(
    stats: Arc<Mutex<Statistics>>,
    env: Environment,
    files: Vec<PathBuf>,
) -> anyhow::Result<AssetCollection> {
    let assets = Arc::new(Mutex::new(AssetCollection::default()));

    thread::scope(|s| {
        for file in files {
            let assets = assets.clone();
            let stats = stats.clone();

            s.spawn(move || {
                let span = span!(Level::TRACE, "build");
                let _enter = span.enter();
                let file_name = file.display().to_string();

                let data = match load_file_payload(env, file) {
                    (_, Some(data)) => data,
                    (skipped, None) => {
                        let mut stats = stats.lock().unwrap();
                        if skipped {
                            stats.skipped += 1;
                        } else {
                            stats.failed += 1;
                        }
                        return;
                    }
                };

                let mut assets = assets.lock().unwrap();
                let success = match data {
                    FilePayload::Tunnel(tunnel) => {
                        assets.tunnels.push(tunnel.build());
                        true
                    }
                    FilePayload::Room(room) => match room.build() {
                        Ok(room) => {
                            assets.rooms.push(room);
                            true
                        }
                        Err(err) => {
                            tracing::warn!(file = file_name, "{err}\n");
                            false
                        }
                    },
                };

                let mut stats = stats.lock().unwrap();
                if success {
                    stats.succeeded += 1;
                } else {
                    stats.failed += 1;
                }
            });
        }
    });

    let assets = Arc::try_unwrap(assets)
        .map_err(|_| anyhow!("unwrapping assets failed"))?
        .into_inner()?;

    Ok(assets)
}

fn load_file_payload(env: Environment, file: PathBuf) -> (bool, Option<FilePayload>) {
    let fail = |step: &str, error: &anyhow::Error| {
        warn!(
            file = file.display().to_string(),
            step,
            error = error.to_string(),
            "fail"
        );
    };

    let text = match read_file(&file) {
        Ok(data) => data,
        Err(error) => {
            fail("read", &error);
            return (false, None);
        }
    };
    let data = match deserialize_file(text) {
        Ok(data) => data,
        Err(error) => {
            fail("deserialize", &error);
            return (false, None);
        }
    };
    if !data.environment().should_include_for(env) {
        debug!(
            file = file.display().to_string(),
            step = "filter_by_environment",
            "skip"
        );
        return (true, None);
    }

    (false, Some(data))
}

fn read_file(file: &Path) -> anyhow::Result<String> {
    let mut file = File::open(file)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;

    Ok(text)
}

fn deserialize_file(text: String) -> anyhow::Result<FilePayload> {
    let data = ron::from_str(&text)?;

    Ok(data)
}
