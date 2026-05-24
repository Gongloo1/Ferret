use anyhow::Result;
use bstr::ByteSlice;
use clap::Parser;
use memmap2::Mmap;
use rayon::prelude::*;
use serde::Deserialize;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

// ANSI Escape sequence for TrueColor RGB (64, 64, 64) mapping exactly to #404040
const GRAY_START: &str = "\x1b[90m";
const COLOR_RESET: &str = "\x1b[0m";

const BANNER: &str = r#"
███████╗███████╗██████╗ ██████╗ ███████╗████████╗
██╔════╝██╔════╝██╔══██╗██╔══██╗██╔════╝╚══██╔══╝
█████╗  █████╗  ██████╔╝██████╔╝█████╗     ██║   
██╔══╝  ██╔══╝  ██╔══██╗██╔══██╗██╔══╝     ██║   
██║     ███████╗██║  ██║██║  ██║███████╗   ██║   
╚═╝     ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝   ╚═╝   "#;

const SUBTITLE: &str = "FERRET — Fast Efficient Recursive Regex Engine for Text";

#[derive(Parser)]
struct Cli {
    // Made optional so they can fall back to the config file if missing on the command line
    pattern: Option<String>,
    path: Option<PathBuf>,

    // Custom configuration file path option
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

// Maps your TOML structure to a usable Rust object via Serde
#[derive(Deserialize, Default)]
struct Config {
    search: Option<SearchConfig>,
}

#[derive(Deserialize, Default)]
struct SearchConfig {
    pattern: Option<String>,
    path: Option<PathBuf>,
}

fn main() {
    // 1. SIGNAL HANDLING: Intercept Ctrl+C immediately
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    if let Err(_) = ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = writeln!(handle, "\n[!] Operation cancelled by user.");
        process::exit(130); 
    }) {
        eprintln!("Error: Could not initialize OS signal handlers.");
        process::exit(exitcode::SOFTWARE);
    }

    // 2. PARSE COMMAND LINE ARGUMENTS
    let args = Cli::parse();
    let mut pattern = args.pattern;
    let mut path = args.path;

    // 3. INCORPORATE CONFIGURATION FILE FALLBACKS
    if args.config.exists() {
        match fs::read_to_string(&args.config) {
            Ok(content) => {
                match toml::from_str::<Config>(&content) {
                    Ok(config) => {
                        if let Some(search) = config.search {
                            if pattern.is_none() { pattern = search.pattern; }
                            if path.is_none() { path = search.path; }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: Configuration file formatting is invalid: {}", e);
                        process::exit(exitcode::CONFIG);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: Unable to access configuration file: {}", e);
                process::exit(exitcode::NOINPUT);
                    }
                }
            }

    // 4. INTERACTIVE MODE: Prompt if arguments are missing
    if pattern.is_none() || path.is_none() {
        print_banner();
        println!(); 

        if pattern.is_none() {
            print!("> Enter search pattern: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let trimmed = input.trim().to_string();
            if !trimmed.is_empty() {
                pattern = Some(trimmed);
            }
        }

        if path.is_none() {
            print!("> Enter target directory path: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let trimmed = input.trim().to_string();
            if !trimmed.is_empty() {
                // Normalize slashes based on the running OS environment
                path = Some(normalize_path_string(&trimmed));
            }
        }
    }

    // Double-check that we successfully got a target pattern and path from somewhere
    let (final_pattern, mut final_path) = match (pattern, path) {
        (Some(p), Some(t)) => (p, t),
        _ => {
            eprintln!("Error: Missing required search variables.");
            eprintln!("Provide a pattern and path via CLI arguments or configure them inside a TOML file.");
            process::exit(exitcode::USAGE);
        }
    };

    // Clean up CLI or Config paths to match the host OS architecture
    final_path = normalize_path_buf(final_path);
    let pattern_bytes = final_pattern.as_bytes();

    // 5. COLLECT FILES VIA TRAVERSAL
    let files: Vec<PathBuf> = WalkDir::new(&final_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();

    let match_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let match_counter = match_count.clone();

    println!("\nScanning files...\n");

    // 6. PARALLEL EXECUTION WITH ASYNC EXIT LOOPS
    files.par_iter().for_each(|file_path| {
        if !running.load(Ordering::SeqCst) {
            return;
        }

        if let Ok(found_any) = search_in_file(file_path, pattern_bytes) {
            if found_any {
                match_counter.fetch_add(1, Ordering::SeqCst);
            }
        }
    });

    // 7. FINALIZE STATUS CODES DETERMINATION
    if match_count.load(Ordering::SeqCst) > 0 {
        process::exit(exitcode::OK);
    } else {
        process::exit(1); 
    }
}

fn search_in_file(file_path: &PathBuf, pattern: &[u8]) -> Result<bool> {
    let file = File::open(file_path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let file_bytes = &mmap[..];

    let mut line_num = 1;
    let mut found_match = false;
    
    for mut line in file_bytes.split_str(b"\n") {
        // Cross-Platform Polish: Strip trailing Windows carriage returns (\r) if present
        if line.ends_with(b"\r") {
            line = &line[..line.len() - 1];
        }

        if line.contains_str(pattern) {
            found_match = true;
            let stdout = io::stdout();
            let mut handle = stdout.lock();

            // Display path cleanly with correct system slashes 
            let _ = write!(handle, "{}:{}: {}", file_path.display(), line_num, line.to_str_lossy());
            if !line.ends_with(b"\n") {
                let _ = writeln!(handle);
            }
        }
        line_num += 1;
    }

    Ok(found_match)
}

fn print_banner() {
    println!("{}{}{}", GRAY_START, BANNER, COLOR_RESET);
    println!("{}", SUBTITLE);
}

/// Helper to sanitize a path string into an OS-appropriate PathBuf layout
fn normalize_path_string(path_str: &str) -> PathBuf {
    let normalized = if cfg!(windows) {
        path_str.replace('/', "\\")
    } else {
        path_str.replace('\\', "/")
    };
    PathBuf::from(normalized)
}

/// Helper to ensure PathBuf structures loaded via TOML or CLI are normalized for the current OS
fn normalize_path_buf(path: PathBuf) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        normalize_path_string(path_str)
    } else {
        path
    }
}