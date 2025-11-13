use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use regex::Regex;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};

mod cat_api;
mod display;

#[derive(Parser, Debug, Clone)]
#[command(name = "catlog")]
#[command(about = "Monitor logs and display cat images on HTTP errors", long_about = None)]
struct Args {
    /// Follow a file (like tail -f)
    #[arg(short, long)]
    follow: Option<String>,

    /// Execute a command and monitor its output
    #[arg(short, long)]
    exec: Option<String>,

    /// Image size in characters
    #[arg(long, default_value = "60")]
    size: u32,

    /// Don't display images
    #[arg(long)]
    no_image: bool,

    /// Only show cats for errors (4xx, 5xx)
    #[arg(long, default_value = "true")]
    errors_only: bool,

    /// Show cats for all status codes
    #[arg(long)]
    all: bool,

    /// Comma-separated list of specific status codes to match
    #[arg(long)]
    status: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Parse status code filter
    let status_filter = args.status.as_ref().map(|codes| {
        codes
            .split(',')
            .filter_map(|s| s.trim().parse::<u16>().ok())
            .collect::<Vec<_>>()
    });

    // Determine which mode to run
    if let Some(ref file_path) = args.follow {
        follow_file(file_path, &args, status_filter.as_deref()).await?;
    } else if let Some(ref command) = args.exec {
        exec_command(command, &args, status_filter.as_deref()).await?;
    } else {
        // Pipe mode: read from stdin
        pipe_mode(&args, status_filter.as_deref()).await?;
    }

    Ok(())
}

async fn pipe_mode(args: &Args, status_filter: Option<&[u16]>) -> Result<()> {
    let stdin = io::stdin();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = line?;
        process_line(&line, args, status_filter).await?;
    }

    Ok(())
}

async fn follow_file(file_path: &str, args: &Args, status_filter: Option<&[u16]>) -> Result<()> {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::path::Path;
    use std::sync::mpsc::channel;

    let path = Path::new(file_path);
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();

    // Read existing content first
    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim_end();
        process_line(trimmed, args, status_filter).await?;
        line.clear();
    }

    // Watch for changes
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Config::default())?;
    watcher.watch(path, RecursiveMode::NonRecursive)?;

    // Continue reading new lines
    loop {
        if let Ok(_) = rx.try_recv() {
            // File was modified, read new lines
            while reader.read_line(&mut line)? > 0 {
                let trimmed = line.trim_end();
                process_line(trimmed, args, status_filter).await?;
                line.clear();
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

async fn exec_command(command: &str, args: &Args, status_filter: Option<&[u16]>) -> Result<()> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Process stdout
    let args_clone = args.clone();
    let filter_clone = status_filter.map(|s| s.to_vec());

    let stdout_handle = tokio::task::spawn_blocking(move || {
        for line in stdout_reader.lines() {
            if let Ok(line) = line {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let _ = process_line(&line, &args_clone, filter_clone.as_deref()).await;
                });
            }
        }
    });

    let args_clone2 = args.clone();
    let filter_clone2 = status_filter.map(|s| s.to_vec());

    let stderr_handle = tokio::task::spawn_blocking(move || {
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let _ = process_line(&line, &args_clone2, filter_clone2.as_deref()).await;
                });
            }
        }
    });

    stdout_handle.await?;
    stderr_handle.await?;
    child.wait()?;

    Ok(())
}

async fn process_line(line: &str, args: &Args, status_filter: Option<&[u16]>) -> Result<()> {
    // Always output the original line
    println!("{}", line);
    io::stdout().flush()?;

    // Detect HTTP status codes
    let re = Regex::new(r"\b([45]\d{2})\b")?;

    if let Some(caps) = re.captures(line) {
        if let Some(code_match) = caps.get(1) {
            let code: u16 = code_match.as_str().parse()?;

            // Check if we should show a cat for this status code
            let should_show = if let Some(filter) = status_filter {
                filter.contains(&code)
            } else if args.all {
                true
            } else {
                // Default: errors only (4xx, 5xx)
                code >= 400 && code < 600
            };

            if should_show {
                display_cat_for_status(code, args).await?;
            }
        }
    }

    Ok(())
}

async fn display_cat_for_status(code: u16, args: &Args) -> Result<()> {
    let message = format!("🐱 {} Detected! 🐱", code);
    println!("\n{}\n", message.bright_yellow().bold());

    if !args.no_image {
        if let Ok(image_data) = cat_api::fetch_cat_image_for_status(code).await {
            display::display_image(&image_data, args.size)?;
        }
    }

    println!();
    Ok(())
}
