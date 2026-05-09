use clap::Args;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write, BufWriter};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Args, Debug)]
pub struct CatArgs {
    /// Number all output lines
    #[arg(short = 'n', long)]
    pub number_all: bool,

    /// Number non-empty output lines (overrides -n for blank lines)
    #[arg(short = 'b', long)]
    pub number_nonblank: bool,

    /// Squeeze multiple adjacent blank lines into one
    #[arg(short = 's', long)]
    pub squeeze_blank: bool,

    /// Files to concatenate. Reads from stdin if empty.
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,
}

#[derive(Error, Debug)]
pub enum CatError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Returns `true` if any error occurred during processing.
/// POSIX behavior: continue processing remaining files even if one fails.
pub fn run(args: CatArgs) -> bool {
    let mut had_error = false;

    // Setup input streams: stdin or files
    let inputs: Vec<(String, Box<dyn BufRead>)> = if args.files.is_empty() {
        vec![(String::from("<stdin>"), Box::new(BufReader::new(io::stdin().lock())))]
    } else {
        args.files
            .into_iter()
            .filter_map(|p| {
                let path_str = p.display().to_string();
                match File::open(&p) {
                    Ok(f) => Some((path_str, Box::new(BufReader::new(f)) as Box<dyn BufRead>)),
                    Err(e) => {
                        eprintln!("RsCoreUtils: cat: {path_str}: {e}");
                        had_error = true;
                        None
                    }
                }
            })
            .collect()
    };

    let mut out = BufWriter::new(io::stdout().lock());
    let mut line_buf = String::with_capacity(4096);
    let mut line_num: u64 = 1;
    let mut prev_blank = false;

    for (path, mut input) in inputs {
        loop {
            line_buf.clear();
            match input.read_line(&mut line_buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let is_blank = line_buf.trim().is_empty();

                    // -s: squeeze consecutive blank lines
                    if args.squeeze_blank && is_blank && prev_blank {
                        continue;
                    }
                    prev_blank = is_blank;

                    // Decide whether to number this line
                    let should_number = if args.number_nonblank {
                        !is_blank
                    } else if args.number_all {
                        true
                    } else {
                        false
                    };

                    if should_number {
                        if write!(out, "{:>6}\t", line_num).is_err() {
                            had_error = true;
                            break;
                        }
                        line_num += 1;
                    }

                    if out.write_all(line_buf.as_bytes()).is_err() {
                        had_error = true;
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("RsCoreUtils: cat: {path}: read error: {e}");
                    had_error = true;
                    break; // Skip to next file on read failure
                }
            }
        }
    }

    if out.flush().is_err() {
        eprintln!("RsCoreUtils: cat: write error on stdout");
        had_error = true;
    }

    had_error
}