
use std::{
    collections::VecDeque,
    process,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::utils::sanitize_filename;



/// #panics when mutex is poisoned
pub fn worker_function(queue: Arc<Mutex<VecDeque<String>>>, template: String) {
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

pub fn spawn_workers(
    count: usize,
    queue: Arc<Mutex<VecDeque<String>>>,
    template: String,
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
