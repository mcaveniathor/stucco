//! `site <out> [--base /stucco/] [--wasm path/to/site_wasm.wasm]`

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut out = None;
    let mut options = site::Options {
        base: "/".into(),
        wasm: None,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base" => options.base = args.next().unwrap_or_default(),
            "--wasm" => options.wasm = args.next().map(PathBuf::from),
            _ if out.is_none() => out = Some(PathBuf::from(arg)),
            _ => {
                eprintln!("unexpected argument: {arg}");
                return ExitCode::FAILURE;
            }
        }
    }
    let Some(out) = out else {
        eprintln!("usage: site <out> [--base /stucco/] [--wasm site_wasm.wasm]");
        return ExitCode::FAILURE;
    };
    match site::build(&out, &options) {
        Ok(files) => {
            println!("wrote {} files to {}", files.len(), out.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build the site: {error}");
            ExitCode::FAILURE
        }
    }
}
