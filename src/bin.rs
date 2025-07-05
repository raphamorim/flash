/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use flash::interpreter::Interpreter;
use std::env;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut interpreter = Interpreter::new();

    let args: Vec<String> = env::args().collect();

    // Check if stdin is a terminal first
    let is_tty = unsafe { libc::isatty(0) } == 1;

    // If stdin is not a TTY, check for piped input first
    if !is_tty {
        // Not a terminal, read from stdin
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;

        if !input.trim().is_empty() {
            // If there were command line arguments, treat them as script arguments
            if args.len() > 1 {
                // Set up arguments for the piped script
                let mut script_args = vec!["flash".to_string()]; // $0
                script_args.extend_from_slice(&args[1..]);
                interpreter.set_args(script_args);
            }

            match interpreter.execute(&input) {
                Ok(exit_code) => std::process::exit(exit_code),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        // If no piped input but there are args, fall through to script file handling
    }

    // If there are command line arguments (other than the program name)
    if args.len() > 1 {
        // Check if it's a -c flag for direct command execution
        if args[1] == "-c" && args.len() > 2 {
            // Execute the command directly: flash -c "command"
            let command = &args[2];
            match interpreter.execute(command) {
                Ok(exit_code) => std::process::exit(exit_code),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        } else {
            // Assume first argument is a script file, rest are script arguments
            let script_path = &args[1];

            // Set up arguments: $0 = script path, $1 = first arg, etc.
            interpreter.set_args(args[1..].to_vec());

            // Try to read and execute the script file
            match std::fs::read_to_string(script_path) {
                Ok(script_content) => match interpreter.execute(&script_content) {
                    Ok(exit_code) => std::process::exit(exit_code),
                    Err(e) => {
                        eprintln!("Error executing script {script_path}: {e}");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error reading script {script_path}: {e}");
                    std::process::exit(1);
                }
            }
        }
    }

    // Interactive mode
    interpreter.run_interactive()?;
    Ok(())
}
