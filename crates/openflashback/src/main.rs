//! OpenFlashback: plays Flashback (1992) from the data files of your own copy.

mod app;
mod tools;

use of_engine::DataDir;
use std::path::PathBuf;
use std::process::ExitCode;

/// Command-line options for playing.
pub struct Options {
    pub data: DataDir,
    pub windowed: bool,
    /// Level to start in directly (0-based), skipping the title screen.
    pub level: Option<usize>,
    pub room: Option<usize>,
}

/// The data directory: `--data`, `OPENFLASHBACK_DATA`, or the executable's / working directory.
fn find_data(explicit: Option<PathBuf>) -> Result<DataDir, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = explicit {
        candidates.push(p);
    } else {
        if let Ok(p) = std::env::var("OPENFLASHBACK_DATA") {
            candidates.push(p.into());
        }
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(PathBuf::from)) {
            candidates.push(dir);
        }
        candidates.push(PathBuf::from("."));
    }
    for c in &candidates {
        if let Ok(d) = DataDir::open(c) {
            return Ok(d);
        }
    }
    Err(format!(
        "Flashback data files not found (looked in {}). Copy the DATA folder of your Flashback install \
         next to openflashback, or pass --data <dir>.",
        candidates
            .iter()
            .map(|c| c.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn usage() {
    eprintln!(
        "usage: openflashback [--data DIR] [--windowed] [--level N] [--room N]\n\
         tools: openflashback <tool> ...\n{}",
        tools::HELP
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| tools::is_tool(a)) {
        return match tools::run(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }

    let mut data = None;
    let mut windowed = false;
    let mut level = None;
    let mut room = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--data" => data = it.next().map(PathBuf::from),
            "--windowed" => windowed = true,
            "--level" => {
                level = it
                    .next()
                    .and_then(|v| v.parse::<usize>().ok())
                    .map(|n| n.saturating_sub(1))
            }
            "--room" => room = it.next().and_then(|v| v.parse().ok()),
            "-h" | "--help" => {
                usage();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown option {other}");
                usage();
                return ExitCode::FAILURE;
            }
        }
    }
    let data = match find_data(data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match app::run(Options {
        data,
        windowed,
        level,
        room,
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
