use std::{
    io::{self, BufRead},
    process,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "multirun",
    version = "0.1",
    about = "Tiny parallel command executor"
)]

struct Arguments {
    #[arg(long = "workers", default_value_t = 4)]
    workers: usize,
    template: String,
}

// ==================== THREADPOOL IMPLEMENTATION ====================

#[derive(Debug)]
struct PoolCreationError(String);

impl std::fmt::Display for PoolCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PoolCreationError {}

// for woker closure

type Job = Box<dyn FnOnce() + Send + 'static>;

struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    fn new(size: usize) -> Result<ThreadPool, PoolCreationError> {
        if size == 0 {
            return Err(PoolCreationError(
                "ThreadPool size must be greater than 0".to_string(),
            ));
        }
        let (sender, reciever) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(reciever));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            let element = Worker::new(id, Arc::clone(&receiver))?;
            workers.push(element);
        }

        Ok(ThreadPool {
            workers,
            sender: Some(sender),
        })
    }

    fn execute<F>(&self, worker_function: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(worker_function);

        // let sender_ref = self.sender.as_ref().expect("no sender in threadpool");

        // sender_ref.send(job).expect("couldn't send job to channel");
        match &self.sender {
            Some(sender) => {
                if let Err(err) = sender.send(job) {
                    eprintln!("ThreadPool: failed to send job: {}", err);
                }
            }
            None => {
                eprintln!("ThreadPool: sender is None, cannot send job");
            }
        }
    }

}



impl Drop for ThreadPool {

    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(handle) = worker.thread.take() {
                // handle.join().unwrap();
                if let Err(err) = handle.join() {
                    eprintln!("Failed to join worker {}: {:?}", worker.id, err);
                }
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Result<Worker, PoolCreationError> {
        let handle = thread::spawn(move || {
            loop {
                let job = receiver.lock().expect("poisoned data").recv();
                match job {
                    Ok(job) => job(),
                    Err(_) => break,
                }
            }
        });

        Ok(Worker {
            id,
            thread: Some(handle),
        })
    }
}

// ==================== UTILITY FUNCTIONS ====================

pub fn read_inputs() -> Result<Vec<(usize, String)>, io::Error> {
    if atty::is(atty::Stream::Stdin) {
        eprintln!("No input detected on stdin.");
        eprintln!("Example: seq 1 5 | multirun \"echo {{}}\"");
        return Err(io::Error::new(io::ErrorKind::Other, "no stdin"));
    }

    let mut inputs = Vec::new();

    let stdin = io::stdin();
    for (i, path) in stdin.lock().lines().enumerate() {
        let p = path?;
        inputs.push((i + 1, p));
    }

    Ok(inputs)
}

pub fn sanitize_filename(index: usize) -> String {
    format!("output{}", index)
}

//=========================================================================================

fn spawn_workers(
    size: usize,
    inputs: Vec<(usize, String)>,
    template: String,
) -> Result<ThreadPool, PoolCreationError> {
    let pool = ThreadPool::new(size)?;

    for (index, path) in inputs {
        let template_clone = template.clone();

        pool.execute(move || process_task(template_clone, index, path));
    }

    Ok(pool)
}

fn process_task(template: String, index: usize, path: String) {
    let output_filename = sanitize_filename(index);
    let final_cmd = template
        .replace("{}", &path)
        .replace("{out}", &output_filename);
    let mut final_cmd_parts = final_cmd.split_whitespace();

    let program_command = match final_cmd_parts.next() {
        Some(c) => c,
        None => {
            eprintln!("[{}] empty command, skipping", path);
            return;
        }
    };

    let program_arguments: Vec<&str> = final_cmd_parts.collect();

    let status = process::Command::new(program_command)
        .args(program_arguments)
        .status();
    match status {
        Ok(s) => match s.code() {
            Some(code) => {
                eprintln!("[{}] exited code: {}", path, code);
            }
            None => {
                eprint!("[{}] terminated by signal", path);
            }
        },
        Err(e) => {
            eprint!("[{}] failed execution: {}", path, e);
        }
    }
}

fn main() {
    let args = Arguments::parse();

    let queue = match read_inputs() {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Failed to read input from stdin: {}", e);
            return;
        }
    };

    println!("no. of workers = {}", args.workers);

    let _pool = match spawn_workers(args.workers, queue, args.template) {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Failed to create thread pool: {}", e);
            return;
        }
    };
}
