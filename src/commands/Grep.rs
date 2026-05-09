use clap::Args;
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write, BufWriter, IsTerminal};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Args, Debug)]
pub struct GrepArgs {
    /// Pattern to search for
    pub pattern: String,

    /// Files to search. Reads from stdin if empty.
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Interpret pattern as fixed string (disables regex)
    #[arg(short = 'F', long)]
    pub fixed_strings: bool,

    /// Ignore case distinctions
    #[arg(short = 'i', long)]
    pub ignore_case: bool,

    /// Invert match: print non-matching lines
    #[arg(short = 'v', long)]
    pub invert_match: bool,

    /// Print line numbers
    #[arg(short = 'n', long)]
    pub line_number: bool,

    /// Colorize output: auto, always, never
    #[arg(long, default_value = "auto")]
    pub color: String,
}

#[derive(Error, Debug)]
pub enum GrepError {
    #[error("Regex compilation failed: {0}")]
    Regex(#[from] regex::Error),
}

enum ColorMode { Auto, Always, Never }

impl std::str::FromStr for ColorMode {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(ColorMode::Auto),
            "always" => Ok(ColorMode::Always),
            "never" => Ok(ColorMode::Never),
            _ => Err("invalid value"),
        }
    }
}

/// Returns POSIX exit code: 0 (match), 1 (no match), 2 (error)
pub fn run(args: GrepArgs) -> u8 {
    // 1. Color mode
    let color_mode = args.color.parse::<ColorMode>().unwrap_or(ColorMode::Never);
    let use_color = match color_mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => io::stdout().is_terminal(), // Stable since Rust 1.70
    } && !args.invert_match; // -v non highlighta (nessun match da evidenziare)

    // 2. Compile pattern (escape if fixed string)
    let pat = if args.fixed_strings {
        regex::escape(&args.pattern)
    } else {
        args.pattern.clone()
    };

    let re = match regex::RegexBuilder::new(&pat)
        .case_insensitive(args.ignore_case)
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("RsCoreUtils: grep: {e}");
            return 2;
        }
    };

    // 3. Setup input streams
    let mut had_error = false;
    let inputs: Vec<(String, Box<dyn BufRead>)> = if args.files.is_empty() {
        vec![(String::from("<stdin>"), Box::new(BufReader::new(io::stdin().lock())) as Box<dyn BufRead>)]
    } else {
        args.files
            .into_iter()
            .filter_map(|p| {
                let name = p.display().to_string();
                match File::open(&p) {
                    Ok(f) => Some((name, Box::new(BufReader::new(f)) as Box<dyn BufRead>)),
                    Err(e) => {
                        eprintln!("RsCoreUtils: grep: {name}: {e}");
                        had_error = true;
                        None
                    }
                }
            })
            .collect()
    };

    // 4. Processing loop
    let mut had_match = false;
    let mut out = BufWriter::new(io::stdout().lock());
    let mut line_buf = String::with_capacity(4096); // Riutilizzo heap

    for (_name, mut input) in inputs {
        let mut line_num: u64 = 1;
        loop {
            line_buf.clear();
            match input.read_line(&mut line_buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    // XOR logico per gestire -v
                    let matches = re.is_match(&line_buf) ^ args.invert_match;
                    if matches {
                        had_match = true;
                        if args.line_number {
                            if write!(out, "{line_num}:").is_err() { had_error = true; break; }
                        }
                        if use_color {
                            if write_colored(&mut out, &line_buf, &re).is_err() { had_error = true; break; }
                        } else {
                            if out.write_all(line_buf.as_bytes()).is_err() { had_error = true; break; }
                        }
                    }
                    line_num += 1;
                }
                Err(e) => {
                    eprintln!("RsCoreUtils: grep: read error: {e}");
                    had_error = true;
                    break;
                }
            }
        }
        if had_error { break; }
    }

    if out.flush().is_err() {
        had_error = true;
    }

    // POSIX exit codes
    if had_error { 2 } else if !had_match { 1 } else { 0 }
}

/// Zero-allocation ANSI highlighting.
/// Writes directly row's bytes, injecting escape sequences only around matches.
fn write_colored(out: &mut impl Write, line: &str, re: &Regex) -> io::Result<()> {
    let mut last_end = 0;
    for m in re.find_iter(line) {
        out.write_all(line[last_end..m.start()].as_bytes())?;
        out.write_all(b"\x1b[01;31m")?; // Bright Red
        out.write_all(line[m.start()..m.end()].as_bytes())?;
        out.write_all(b"\x1b[0m")?;     // Reset
        last_end = m.end();
    }
    out.write_all(line[last_end..].as_bytes())
}