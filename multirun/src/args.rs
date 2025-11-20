use clap::Parser;
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
pub struct Arguments {
    /// Number of parallel workers to spawn.
    #[arg(long = "workers", default_value_t = 4)]
    pub workers: usize,

    /// Command template. Use `{}` as placeholder for each line of input.
    ///
    /// Example:  "echo processing {}"
    #[arg(value_parser = non_empty_string)]
    pub template: String,
}

// validator function
pub fn non_empty_string(val: &str) -> Result<String, String> {
    if val.trim().is_empty() {
        Err("template must be a non-empty string".to_string())
    } else if !val.contains("{}") {
        Err("template must contain {} parameter".to_string())
    } else {
        Ok(val.to_string())
    }
}