use clap::Parser;
use std::{
    collections::VecDeque,
    io::{self, BufRead},
    process,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

/// Run a command on multiple inputs in parallel, similar to GNU parallel.
///
/// The special placeholder `{}` is replaced with each input line.
/// The optional `{out}` placeholder is replaced with a sanitized filename.
///
/// Example:
///     seq 1 5 | multirun --workers 4 "echo processing {}"
///
/// Example with output filename:
///     cat files.txt | multirun "gzip {} -c > {out}.gz"
#[derive(Parser, Debug)]
#[command(
    name = "multirun",
    version = "0.1",
    about = "Tiny parallel command executor"
)]
struct Args {
    /// Number of parallel workers to spawn.
    #[arg(long = "workers", default_value_t = 4)]
    workers: usize,

    /// Command template. Use `{}` as placeholder for each line of input.
    ///
    /// Example:  "echo processing {}"
    #[arg(value_parser = non_empty_string)]
    template: String,
}

// validator function
fn non_empty_string(val: &str) -> Result<String, String> {
    if val.trim().is_empty() {
        Err("template must be a non-empty string".to_string())
    } else if !val.contains("{}") {
        Err("template must contain {}".to_string())
    } else {
        Ok(val.to_string())
    }
}

/// Sanitize a string to make a safe filename
fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// #panics when mutex is poisoned
/// #returns Result<>
//  function to read all inputs
fn read_inputs(template: &str) -> Result<Arc<Mutex<VecDeque<String>>>, io::Error> {
    // If template needs input but stdin is a TTY, show help.
    if template.contains("{}") && atty::is(atty::Stream::Stdin) {
        eprintln!("No input detected on stdin.\nExample usage:");
        eprintln!("  seq 1 5 | multirun \"echo {{}}\"");
        eprintln!("  cat files.txt | multirun \"cp {{}} {{out}}\"");
        return Err(io::Error::new(io::ErrorKind::Other, "no stdin"));
    }

    let queue = Arc::new(Mutex::new(VecDeque::new()));

    let stdin = io::stdin();

    for paths in stdin.lock().lines() {
        let l = paths?;
        queue
            .lock()
            .expect("Mutex poisoned while pushing input into queue")
            .push_back(l);
    }

    Ok(queue)
}

/// #panics when mutex is poisoned
fn worker_function(queue: Arc<Mutex<VecDeque<String>>>, template: String) {
    loop {
        let task = queue
            .lock()
            .expect("Mutex poisoned while trying to acquire lock")
            .pop_front();

        let path = match task {
            Some(l) => l,
            None => break,
        };

        let output_filename = sanitize_filename(&path);
        let cmd_final = template
            .replace("{}", &path)
            .replace("{out}", &output_filename);

        let mut cmd_final_parts = cmd_final.split_whitespace();

        let program_command = match cmd_final_parts.next() {
            Some(cmd) => cmd,
            None => {
                eprintln!("[{}] template produced an empty command, skipping", path);
                continue;
            }
        };

        let program_arguments: Vec<&str> = cmd_final_parts.collect();

        let execution = process::Command::new(program_command)
            .args(program_arguments)
            .status();

        match execution {
            Ok(s) => match s.code() {
                Some(code) => eprintln!("[{}] exited code: {}", path, code),
                None => eprintln!("[{}] terminated without exit code (signal?)", path),
            },
            Err(e) => {
                eprintln!("[{}] failed: {}", path, e)
            }
        }
    }
}

fn spawn_workers(
    count: usize,
    queue: Arc<Mutex<VecDeque<String>>>,
    template: String,
    worker_function: fn(Arc<Mutex<VecDeque<String>>>, String),
) -> Vec<JoinHandle<()>> {
    let mut handles = Vec::new();
    for _ in 0..count {
        let queue = Arc::clone(&queue);

        let template_clone = template.clone();

        let handle = thread::spawn(move || worker_function(queue, template_clone));
        handles.push(handle);
    }

    handles
}

fn main() {
    let args = Args::parse();

    let queue = match read_inputs(&args.template) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Failed to read input from stdin: {}", e);
            return;
        }
    };

    let handles = spawn_workers(args.workers, queue, args.template, worker_function);

    println!("j = {}", args.workers);

    // Wait for all threads to finish
    for handle in handles {
        let _ = handle.join();
    }
}
