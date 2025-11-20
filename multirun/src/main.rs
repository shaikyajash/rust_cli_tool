


mod args;
mod utils;
mod worker;


use args::Arguments;
use clap::Parser;

use crate::{utils::read_inputs, worker::{spawn_workers}};

fn main() {
    let args = Arguments::parse();

    let queue = match read_inputs() {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Failed to read input from stdin: {}", e);
            return;
        }
    };

    let handles = spawn_workers(args.workers, queue, args.template);

    println!("j = {}", args.workers);

    // Wait for all threads to finish
    for handle in handles {
        let _ = handle.join();
    }
}
