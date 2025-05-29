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

    // If there are command line arguments (other than the program name), execute them
    if args.len() > 1 {
        let command = args[1..].join(" ");
        match interpreter.execute(&command) {
            Ok(exit_code) => std::process::exit(exit_code),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Check if stdin is a terminal
    if unsafe { libc::isatty(0) } == 0 {
        // Not a terminal, read from stdin
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;

        if !input.trim().is_empty() {
            match interpreter.execute(&input) {
                Ok(exit_code) => std::process::exit(exit_code),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        return Ok(());
    }

    // Interactive mode
    interpreter.run_interactive()?;
    Ok(())
}
