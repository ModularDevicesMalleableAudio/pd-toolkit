mod cli;
mod commands;
mod errors;
mod io;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();
    let verbose = cli.verbose;

    let exit = match cli.command {
        Some(Commands::Parse { file, json, output }) => {
            match commands::parse::run(&file, json, output.as_deref(), verbose) {
                Ok(out) => {
                    println!("{out}");
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    e.exit_code()
                }
            }
        }
        Some(Commands::List { file, depth, json, output }) => {
            match commands::list::run(&file, depth, json, output.as_deref()) {
                Ok(out) => {
                    if !out.is_empty() {
                        println!("{out}");
                    }
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    e.exit_code()
                }
            }
        }
        Some(Commands::Validate { file, strict, json, output }) => {
            match commands::validate::run(&file, strict, json, output.as_deref()) {
                Ok(result) => {
                    if !result.output.is_empty() {
                        println!("{}", result.output);
                    }
                    result.exit_code
                }
                Err(e) => {
                    eprintln!("{e}");
                    e.exit_code()
                }
            }
        }
        Some(Commands::Insert { file, depth, index, entry, in_place, backup, output }) => {
            match commands::insert::run(
                &file, depth, index, &entry, in_place, backup, output.as_deref(),
            ) {
                Ok((serialized, code)) => {
                    if !in_place && output.is_none() {
                        print!("{serialized}");
                    }
                    code
                }
                Err(e) => {
                    eprintln!("{e}");
                    e.exit_code()
                }
            }
        }
        Some(Commands::Delete { file, depth, index, in_place, backup, output }) => {
            match commands::delete::run(&file, depth, index, in_place, backup, output.as_deref()) {
                Ok((serialized, code)) => {
                    if !in_place && output.is_none() {
                        print!("{serialized}");
                    }
                    code
                }
                Err(e) => {
                    eprintln!("{e}");
                    e.exit_code()
                }
            }
        }
        Some(Commands::Renumber { file, depth, from, delta, in_place, backup, output }) => {
            match commands::renumber::run(
                &file, depth, from, delta, in_place, backup, output.as_deref(),
            ) {
                Ok((serialized, code)) => {
                    if !in_place && output.is_none() {
                        print!("{serialized}");
                    }
                    code
                }
                Err(e) => {
                    eprintln!("{e}");
                    e.exit_code()
                }
            }
        }
        None => {
            // Print help then exit 0.
            let _ = Cli::parse_from(["pdtk", "--help"]);
            0
        }
    };

    std::process::exit(exit);
}
