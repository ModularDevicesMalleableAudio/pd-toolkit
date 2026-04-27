mod cli;
mod commands;
mod errors;
mod io;
mod types;

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
        Some(Commands::List {
            file,
            depth,
            json,
            output,
        }) => match commands::list::run(&file, depth, json, output.as_deref()) {
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
        },
        Some(Commands::Validate {
            file,
            strict,
            json,
            output,
        }) => match commands::validate::run(&file, strict, json, output.as_deref()) {
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
        },
        Some(Commands::Insert {
            file,
            depth,
            index,
            entry,
            in_place,
            backup,
            output,
        }) => {
            match commands::insert::run(
                &file,
                depth,
                index,
                &entry,
                in_place,
                backup,
                output.as_deref(),
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
        Some(Commands::Delete {
            file,
            depth,
            index,
            subpatch,
            in_place,
            backup,
            output,
        }) => {
            match commands::delete::run(
                &file,
                depth,
                index,
                subpatch,
                in_place,
                backup,
                output.as_deref(),
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
        Some(Commands::Renumber {
            file,
            depth,
            from,
            delta,
            in_place,
            backup,
            output,
        }) => {
            match commands::renumber::run(
                &file,
                depth,
                from,
                delta,
                in_place,
                backup,
                output.as_deref(),
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
        Some(Commands::FindOrphans {
            target,
            depth,
            json,
            delete,
            in_place,
            backup,
            include_comments,
        }) => match commands::find_orphans::run(
            &target,
            depth,
            json,
            delete,
            in_place,
            backup,
            include_comments,
        ) {
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
        },
        Some(Commands::FindDisplays {
            target,
            depth,
            json,
            delete,
            in_place,
            backup,
            include_unconnected,
            include_labels,
        }) => match commands::find_displays::run(commands::find_displays::RunArgs {
            target: &target,
            depth,
            json,
            delete,
            in_place,
            backup,
            include_unconnected,
            include_labels,
        }) {
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
        },
        Some(Commands::Search {
            target,
            obj_type,
            text,
            depth,
            json,
            regex,
            case_sensitive,
        }) => match commands::search::run(
            &target,
            obj_type.as_deref(),
            text.as_deref(),
            depth,
            json,
            regex,
            case_sensitive,
        ) {
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
        },
        Some(Commands::Arrays { target, json }) => match commands::arrays::run(&target, json) {
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
        },
        Some(Commands::Stats { target, json }) => match commands::stats::run(&target, json) {
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
        },
        Some(Commands::Modify {
            file,
            depth,
            index,
            text,
            in_place,
            backup,
            output,
        }) => {
            match commands::modify::run(
                &file,
                depth,
                index,
                &text,
                in_place,
                backup,
                output.as_deref(),
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
        Some(Commands::Connect {
            file,
            depth,
            src,
            outlet,
            dst,
            inlet,
            in_place,
            backup,
            output,
        }) => {
            match commands::connect::run(commands::connect::RunArgs {
                file: &file,
                depth,
                src,
                outlet,
                dst,
                inlet,
                in_place,
                backup,
                output: output.as_deref(),
            }) {
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
        Some(Commands::Disconnect {
            file,
            depth,
            src,
            outlet,
            dst,
            inlet,
            in_place,
            backup,
            output,
        }) => {
            match commands::disconnect::run(commands::disconnect::RunArgs {
                file: &file,
                depth,
                src,
                outlet,
                dst,
                inlet,
                in_place,
                backup,
                output: output.as_deref(),
            }) {
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
        Some(Commands::Connections {
            file,
            index,
            depth,
            json,
        }) => match commands::connections::run(&file, index, depth, json) {
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
        },
        Some(Commands::Format {
            file,
            depth,
            grid,
            hpad,
            margin,
            dry_run,
            in_place,
            backup,
            output,
        }) => {
            match commands::format::run(commands::format::RunArgs {
                file: &file,
                depth,
                grid,
                hpad,
                margin,
                dry_run,
                in_place,
                backup,
                output: output.as_deref(),
            }) {
                Ok(out) => {
                    if dry_run || (!in_place && output.is_none()) {
                        print!("{out}");
                    }
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    e.exit_code()
                }
            }
        }
        Some(Commands::Lint {
            file,
            json,
            send_receive,
            fan_out,
            dsp_loop,
        }) => match commands::lint::run(&file, json, send_receive, fan_out, dsp_loop) {
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
        },
        Some(Commands::Extract {
            file,
            depth,
            output,
            in_place,
            backup,
        }) => match commands::extract::run(&file, depth, &output, in_place, backup) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                e.exit_code()
            }
        },
        Some(Commands::Batch {
            dir,
            command,
            glob,
            dry_run,
            continue_on_error,
            json,
        }) => {
            let cmd_refs: Vec<&str> = command.iter().map(String::as_str).collect();
            match commands::batch::run(&dir, &cmd_refs, &glob, dry_run, continue_on_error, json) {
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
        Some(Commands::Trace {
            file,
            from,
            to,
            depth,
            max_hops,
            json,
        }) => match commands::trace::run(&file, from, to, depth, max_hops, json) {
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
        },
        Some(Commands::Diff {
            file_a,
            file_b,
            json,
            ignore_coords,
        }) => match commands::diff::run(&file_a, &file_b, json, ignore_coords) {
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
        },
        Some(Commands::Deps {
            target,
            recursive,
            missing,
            json,
            search_path,
            pd_path,
        }) => match commands::deps::run(&target, recursive, missing, json, &search_path, pd_path) {
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
        },
        Some(Commands::RenameSend {
            target,
            from,
            to,
            in_place,
            backup,
            dry_run,
            force,
        }) => {
            match commands::rename_send::run(&target, &from, &to, in_place, backup, dry_run, force)
            {
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
        Some(Commands::Completions { shell }) => {
            use clap::CommandFactory;
            use clap_complete::generate;
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "pdtk", &mut std::io::stdout());
            0
        }
        None => {
            // Print help then exit 0.
            let _ = Cli::parse_from(["pdtk", "--help"]);
            0
        }
    };

    std::process::exit(exit);
}
