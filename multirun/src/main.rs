use clap::{Parser};
use std::{
    collections::VecDeque, io::{self, BufRead}, process, sync::{Arc, Mutex}, thread
};

/// Parallel CLI runner, similar to GNU parallel
#[derive(Parser, Debug)]
struct Args {
    /// Number of parallel jobs
    #[arg(long = "workers", default_value_t = 4)]
    workers: usize,
    /// Command template, use {} for input and {out} for optional safe filename
    command: String,
}

/// Sanitize a string to make a safe filename
fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

fn main() {
    let args = Args::parse();

    let queue = Arc::new(Mutex::new(VecDeque::new()));

    let stdin = io::stdin();

    for paths in stdin.lock().lines() {
        match paths {
            Ok(l) => queue.lock().unwrap().push_back(l),
            Err(e) => eprintln!("Failed to read line:{}", e),
        }
    }

    let mut handles = Vec::new();

    println!("j value is {}" , args.workers);

    for _ in 0..args.workers {
        let queue = Arc::clone(&queue);
        let command_template = args.command.clone();

        let handle = thread::spawn(move || {
            loop {
                let task = {
                    let mut q = queue.lock().unwrap();
                    q.pop_front()
                };

                let path = match task {
                    Some(l) => l,
                    None => break,
                };

                let output_filename = sanitize_filename(&path);

                // building the command to execute
                let cmd_final = command_template
                    .replace("{}", &path)
                    .replace("{out}", &output_filename);


                let cmd_final_parts: Vec<&str> = cmd_final.split_whitespace().collect();

                let program_cmd = cmd_final_parts[0];

                //borrowing as we wouldn't know the size at compile time
                let program_arguments  = &cmd_final_parts[1..];
                
                
                match process::Command::new(program_cmd).args(program_arguments).status() {
                Ok(s) => eprintln!("[{}] exit: {:?}", path, s.code()),
                Err(err) => eprintln!("[{}] failed: {}", path, err),
            }

            }
        });

        handles.push(handle);
    }

    // Wait for all threads to finish
    for handle in handles {
        let _ = handle.join();
    }
}
