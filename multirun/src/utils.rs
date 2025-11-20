
use std::{
    collections::VecDeque,
    io::{self, BufRead},
    sync::{Arc, Mutex},
};


/// Sanitize a string to make a safe filename
pub fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}


/// #panics when mutex is poisoned
/// #returns Result<>
//  function to read all inputs
pub fn read_inputs() -> Result<Arc<Mutex<VecDeque<String>>>, io::Error> {
    // If template needs input but stdin is a TTY, show help.
    if atty::is(atty::Stream::Stdin) {
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
