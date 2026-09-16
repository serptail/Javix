mod emit;
mod error;
mod reader;

use std::env;
use std::fs;
use std::process::ExitCode;
use std::sync::atomic::Ordering;

const DEFAULT_INPUT: &str = "Test.class";

const USAGE: &str = "\
usage: javix [options] <file.class>

options:
  --cfg     include each method's control flow graph as a Graphviz comment
  -h        show this message
";

fn main() -> ExitCode {
    let mut path: Option<String> = None;
    let mut show_cfg = false;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--cfg" => show_cfg = true,
            "-h" | "--help" => {
                print!("{}", USAGE);
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("javix: unknown option '{}'", other);
                return ExitCode::FAILURE;
            }
            other => path = Some(other.to_string()),
        }
    }

    emit::SHOW_CFG.store(show_cfg, Ordering::Relaxed);

    let path = path.unwrap_or_else(|| DEFAULT_INPUT.to_string());

    let raw = match fs::read(&path) {
        Ok(raw) => raw,
        Err(err) => {
            eprintln!("javix: couldn't read '{}': {}", path, err);
            return ExitCode::FAILURE;
        }
    };

    let class = match reader::ClassFile::new(&raw) {
        Ok(class) => class,
        Err(err) => {
            eprintln!("javix: couldn't parse '{}': {}", path, err);
            return ExitCode::FAILURE;
        }
    };

    print!("{}", emit::generate_source(&class));
    ExitCode::SUCCESS
}
