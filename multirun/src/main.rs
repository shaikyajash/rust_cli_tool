use clap::{Parser};
use std::{
    collections::VecDeque,
    io::{self, BufRead},
    process,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

/// Parallel CLI runner, similar to GNU parallel
#[derive(Parser, Debug)]
struct Args {
    /// Number of parallel jobs
    #[arg(long = "workers", default_value_t = 4)]
    workers: usize,
    /// Command template, use {} for input and {out} for optional safe filename
    template: String,
}

/// Sanitize a string to make a safe filename
fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

//  function to read all inputs
fn read_inputs() -> Arc<Mutex<VecDeque<String>>> {
    let queue = Arc::new(Mutex::new(VecDeque::new()));

    let stdin = io::stdin();

    for paths in stdin.lock().lines() {
        match paths {
            Ok(l) => {
                queue.lock().unwrap().push_back(l);
            }
            Err(e) => {
                eprintln!("Error while reading the from buffer: {}", e)
            }
        }
    }
    queue
}

fn worker_function(queue: Arc<Mutex<VecDeque<String>>>, template: String) {
    loop {
        let task = queue.lock().unwrap().pop_front();

        let path = match task {
            Some(l) => l,
            None => break,
        };

        let output_filename = sanitize_filename(&path);
        let cmd_final = template
            .replace("{}", &path)
            .replace("{out}", &output_filename);
        let cmd_final_parts: Vec<&str> = cmd_final.split_whitespace().collect();
        let program_command = cmd_final_parts[0];

        // using "&" and borrowing as the compiler doesn't the size here
        let program_arguments = &cmd_final_parts[1..];

        let execution = process::Command::new(program_command)
            .args(program_arguments)
            .status();

        match execution {
            Ok(s) => {
                eprintln!("[{}] exit: {:?}", path, s.code())
            }
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
    worker_function:fn(Arc<Mutex<VecDeque<String>>>, String)
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

    let queue = read_inputs();

    let handles = spawn_workers(args.workers, queue,args.template , worker_function);

    println!("j = {}" , args.workers);

    // Wait for all threads to finish
    for handle in handles {
        let _ = handle.join();
    }

    
}
