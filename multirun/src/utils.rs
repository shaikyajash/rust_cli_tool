
use std::{
    collections::VecDeque,
    io::{self, BufRead},
    sync::{Arc, Mutex},
};


/// Sanitize a string to make a safe filename
pub fn sanitize_filename(index: usize) -> String {
    format!("output{}", index)
}



/// #panics when mutex is poisoned
/// #returns Result<>
//  function to read all inputs
pub fn read_inputs() -> Result<Arc<Mutex<VecDeque<(usize,String)>>>, io::Error> {
    // If template needs input but stdin is a TTY, show help.
    if atty::is(atty::Stream::Stdin) {
        eprintln!("No input detected on stdin.\nExample usage:");
        eprintln!("  seq 1 5 | multirun \"echo {{}}\"");
        eprintln!("  cat files.txt | multirun \"cp {{}} {{out}}\"");
        return Err(io::Error::new(io::ErrorKind::Other, "no stdin"));
    }


    let queue = Arc::new(Mutex::new(VecDeque::new()));

    let stdin = io::stdin();

    for (i,paths) in stdin.lock().lines().enumerate() {
        let l = paths?;

        queue
            .lock()
            .expect("Mutex poisoned while pushing input into queue")
            .push_back((i+1 , l));
    }



    Ok(queue)
}
