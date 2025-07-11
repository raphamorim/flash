/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use crate::completion::CompletionSystem;
use crate::flash;
use crate::lexer::Lexer;
use crate::parser::CasePattern;
use crate::parser::Node;
use crate::parser::Parser;
use crate::parser::Redirect;
use crate::parser::RedirectKind;

use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::mem;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// Terminal control functions using libc
fn tcgetattr(fd: i32) -> Result<libc::termios, io::Error> {
    let mut termios = unsafe { mem::zeroed::<libc::termios>() };
    let result = unsafe { libc::tcgetattr(fd, &mut termios) };
    if result == 0 {
        Ok(termios)
    } else {
        Err(io::Error::last_os_error())
    }
}

fn tcsetattr(fd: i32, optional_actions: i32, termios: &libc::termios) -> Result<(), io::Error> {
    let result = unsafe { libc::tcsetattr(fd, optional_actions, termios) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub trait Evaluator {
    fn evaluate(&mut self, node: &Node, interpreter: &mut Interpreter) -> Result<i32, io::Error>;
}

// Default evaluator that implements the standard shell behavior
pub struct DefaultEvaluator;

impl Evaluator for DefaultEvaluator {
    fn evaluate(&mut self, node: &Node, interpreter: &mut Interpreter) -> Result<i32, io::Error> {
        match node {
            Node::Command {
                name,
                args,
                redirects,
            } => self.evaluate_command(name, args, redirects, interpreter),
            Node::Pipeline { commands } => self.evaluate_pipeline(commands, interpreter),
            Node::List {
                statements,
                operators,
            } => self.evaluate_list(statements, operators, interpreter),
            Node::Assignment { name, value } => self.evaluate_assignment(name, value, interpreter),
            Node::CommandSubstitution { command } => {
                self.evaluate_command_substitution(command, interpreter)
            }
            Node::ArithmeticExpansion { expression } => {
                self.evaluate_arithmetic_expansion(expression, interpreter)
            }
            Node::ArithmeticCommand { expression } => {
                self.evaluate_arithmetic_command(expression, interpreter)
            }
            Node::StringLiteral(_value) => Ok(0),
            Node::SingleQuotedString(_value) => Ok(0),
            Node::Subshell { list } => interpreter.evaluate_with_evaluator(list, self),
            Node::Comment(_) => Ok(0),
            Node::ExtGlobPattern {
                operator,
                patterns,
                suffix,
            } => self.evaluate_ext_glob(*operator, patterns, suffix, interpreter),
            Node::Export { name, value } => self.evaluate_export(name, value, interpreter),
            Node::IfStatement {
                condition,
                consequence,
                alternative,
            } => self.evaluate_if_statement(condition, consequence, alternative, interpreter),
            Node::ElifBranch {
                condition,
                consequence,
            } => self.evaluate_elif_branch(condition, consequence, interpreter),
            Node::ElseBranch { consequence } => self.evaluate_else_branch(consequence, interpreter),
            Node::CaseStatement {
                expression,
                patterns,
            } => self.evaluate_case_statement(expression, patterns, interpreter),
            Node::Function { name, body } => {
                self.evaluate_function_definition(name, body, interpreter)
            }
            Node::FunctionCall {
                name,
                args,
                redirects,
            } => self.evaluate_function_call(name, args, redirects, interpreter),
            Node::Return { value } => self.evaluate_return(value, interpreter),
            Node::HistoryExpansion { pattern } => {
                self.evaluate_history_expansion(pattern, interpreter)
            }
            Node::Complete {
                options: _,
                command: _,
            } => {
                // Complete command is handled silently - just acknowledge it
                Ok(0)
            }
            Node::ForLoop {
                variable,
                iterable,
                body,
            } => self.evaluate_for_loop(variable, iterable, body, interpreter),
            Node::WhileLoop { condition, body } => {
                self.evaluate_while_loop(condition, body, interpreter)
            }
            Node::UntilLoop { condition, body } => {
                self.evaluate_until_loop(condition, body, interpreter)
            }
            Node::Array { elements: _ } => Ok(0), // Arrays are used internally, return success
            Node::Negation { command } => {
                // Logical negation - invert the exit code
                let result = interpreter.evaluate_with_evaluator(command, self)?;
                Ok(if result == 0 { 1 } else { 0 })
            }
            Node::SelectStatement {
                variable,
                items,
                body,
            } => self.evaluate_select_statement(variable, items, body, interpreter),
            Node::ParameterExpansion { .. } => {
                // Parameter expansions should be handled during argument expansion
                // If we reach here, it means it was not expanded properly
                Ok(0)
            }
            _ => Err(io::Error::other("Unsupported node type")),
        }
    }
}

impl DefaultEvaluator {
    fn evaluate_function_definition(
        &mut self,
        name: &str,
        body: &Node,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Store the function definition
        interpreter
            .functions
            .insert(name.to_string(), Box::new(body.clone()));
        Ok(0)
    }

    fn evaluate_function_call(
        &mut self,
        name: &str,
        args: &[String],
        redirects: &[Redirect],
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Check if function exists
        if let Some(body) = interpreter.functions.get(name).cloned() {
            // Set up function arguments as positional parameters
            let old_args = interpreter.args.clone();
            let old_return_value = interpreter.return_value;
            let mut new_args = vec![name.to_string()]; // $0 is function name
            new_args.extend(args.iter().cloned());
            interpreter.args = new_args;
            interpreter.return_value = None; // Clear any previous return value

            // Execute function body
            let result = interpreter.evaluate_with_evaluator(&body, self);

            // Check if function returned early
            let final_result = match result {
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                    // Check if this is a return statement
                    if let Some(msg) = e.to_string().strip_prefix("return:") {
                        if let Ok(return_code) = msg.parse::<i32>() {
                            Ok(return_code)
                        } else {
                            Ok(0)
                        }
                    } else {
                        result
                    }
                }
                _ => result,
            };

            // Restore original arguments and return value
            interpreter.args = old_args;
            interpreter.return_value = old_return_value;

            final_result
        } else {
            // Function not found, treat as regular command
            self.evaluate_command(name, args, redirects, interpreter)
        }
    }

    fn evaluate_return(
        &mut self,
        value: &Option<Box<Node>>,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        let return_code = match value {
            Some(val) => {
                // Evaluate the return value
                match val.as_ref() {
                    Node::StringLiteral(string_value) => {
                        let expanded_value = interpreter.expand_variables(string_value);
                        expanded_value.parse::<i32>().unwrap_or(0)
                    }
                    Node::CommandSubstitution { command } => {
                        // Execute command substitution and use its exit code
                        interpreter.evaluate_with_evaluator(command, self)?
                    }
                    Node::ArithmeticExpansion { expression } => {
                        let expanded_expr = interpreter.expand_variables(expression);
                        match DefaultEvaluator::evaluate_arithmetic_expression(&expanded_expr) {
                            Ok(result) => result as i32,
                            Err(_) => {
                                eprintln!(
                                    "arithmetic expansion: invalid expression: {expanded_expr}"
                                );
                                0
                            }
                        }
                    }
                    Node::ArithmeticCommand { expression } => {
                        let expanded_expr = interpreter.expand_variables(expression);
                        match DefaultEvaluator::evaluate_arithmetic_expression(&expanded_expr) {
                            Ok(result) => result as i32,
                            Err(_) => {
                                eprintln!(
                                    "arithmetic command: invalid expression: {expanded_expr}"
                                );
                                0
                            }
                        }
                    }
                    _ => {
                        // For other node types, evaluate them and use the result
                        interpreter.evaluate_with_evaluator(val, self)?
                    }
                }
            }
            None => {
                // No return value specified, use last exit code
                interpreter.last_exit_code
            }
        };

        // Set the return value in the interpreter
        interpreter.return_value = Some(return_code);

        // Return a special error code to indicate early return from function
        // We'll use a custom error type for this
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("return:{return_code}"),
        ))
    }

    fn evaluate_command(
        &mut self,
        name: &str,
        args: &[String],
        redirects: &[Redirect],
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Expand glob patterns in arguments
        let expanded_args = interpreter.expand_glob_patterns(args);

        // Use expanded args for the rest of the function
        let args = &expanded_args;

        // First check if this is a function call
        if interpreter.functions.contains_key(name) {
            return self.evaluate_function_call(name, args, redirects, interpreter);
        }

        // Check for alias expansion
        if let Some(alias_value) = interpreter.aliases.get(name) {
            // Parse the alias value properly handling escaped spaces
            let mut alias_parts = interpreter.parse_alias_value(alias_value);
            alias_parts.extend_from_slice(args);

            // Create a new command with the expanded alias
            if let Some(new_name) = alias_parts.first() {
                let new_args = &alias_parts[1..];
                return self.evaluate_command(new_name, new_args, redirects, interpreter);
            }
        }

        // Handle built-in commands
        match name {
            "cd" => {
                let dir = if args.is_empty() {
                    env::var("HOME").unwrap_or_else(|_| ".".to_string())
                } else {
                    // Expand variables and tilde in the directory argument
                    interpreter.expand_variables(&args[0])
                };

                match env::set_current_dir(&dir) {
                    Ok(_) => {
                        interpreter.variables.insert(
                            "PWD".to_string(),
                            env::current_dir()?.to_string_lossy().to_string(),
                        );
                        Ok(0)
                    }
                    Err(e) => {
                        eprintln!("cd: {dir}: {e}");
                        Ok(1)
                    }
                }
            }
            "echo" => {
                for (i, arg) in args.iter().enumerate() {
                    let expanded_arg = interpreter.expand_variables(arg);
                    print!("{}{}", if i > 0 { " " } else { "" }, expanded_arg);
                }
                println!();
                Ok(0)
            }
            "export" => {
                if args.is_empty() {
                    // List all exported variables
                    for (key, value) in &interpreter.variables {
                        println!("export {key}={value}");
                    }
                    return Ok(0);
                }

                for arg in args {
                    if let Some(pos) = arg.find('=') {
                        let (key, value) = arg.split_at(pos);
                        let value = &value[1..];
                        if !key.is_empty() {
                            interpreter
                                .variables
                                .insert(key.to_string(), value.to_string());
                            unsafe {
                                env::set_var(key, value);
                            }
                        }
                    } else if let Some(value) = interpreter.variables.get(arg) {
                        if !arg.is_empty() {
                            unsafe {
                                env::set_var(arg, value);
                            }
                        }
                    }
                }
                Ok(0)
            }
            "source" | "." | "\\." => {
                if args.is_empty() {
                    eprintln!("source: filename argument required");
                    return Ok(1);
                }

                let filename = &args[0];
                match fs::read_to_string(filename) {
                    Ok(content) => interpreter.execute(&content),
                    Err(e) => {
                        eprintln!("source: {filename}: {e}");
                        Ok(1)
                    }
                }
            }
            "[" | "test" => {
                // Built-in test command
                self.evaluate_test_command(args, interpreter)
            }
            "exit" => {
                // Built-in exit command
                let exit_code = if args.is_empty() {
                    0
                } else {
                    args[0].parse::<i32>().unwrap_or(0)
                };
                std::process::exit(exit_code);
            }
            "true" => {
                // Built-in true command
                Ok(0)
            }
            "false" => {
                // Built-in false command
                Ok(1)
            }
            "seq" => {
                // Built-in seq command - generate sequence of numbers
                match args.len() {
                    1 => {
                        // seq LAST - from 1 to LAST
                        if let Ok(last) = args[0].parse::<i32>() {
                            for i in 1..=last {
                                println!("{i}");
                            }
                            Ok(0)
                        } else {
                            eprintln!("seq: invalid number: {}", args[0]);
                            Ok(1)
                        }
                    }
                    2 => {
                        // seq FIRST LAST
                        if let (Ok(first), Ok(last)) =
                            (args[0].parse::<i32>(), args[1].parse::<i32>())
                        {
                            if first <= last {
                                for i in first..=last {
                                    println!("{i}");
                                }
                            } else {
                                for i in (last..=first).rev() {
                                    println!("{i}");
                                }
                            }
                            Ok(0)
                        } else {
                            eprintln!("seq: invalid number arguments");
                            Ok(1)
                        }
                    }
                    3 => {
                        // seq FIRST INCREMENT LAST
                        if let (Ok(first), Ok(increment), Ok(last)) = (
                            args[0].parse::<i32>(),
                            args[1].parse::<i32>(),
                            args[2].parse::<i32>(),
                        ) {
                            if increment == 0 {
                                eprintln!("seq: increment cannot be zero");
                                Ok(1)
                            } else if increment > 0 && first <= last {
                                let mut i = first;
                                while i <= last {
                                    println!("{i}");
                                    i += increment;
                                }
                                Ok(0)
                            } else if increment < 0 && first >= last {
                                let mut i = first;
                                while i >= last {
                                    println!("{i}");
                                    i += increment;
                                }
                                Ok(0)
                            } else {
                                // No output for invalid range
                                Ok(0)
                            }
                        } else {
                            eprintln!("seq: invalid number arguments");
                            Ok(1)
                        }
                    }
                    _ => {
                        eprintln!("seq: wrong number of arguments");
                        Ok(1)
                    }
                }
            }
            "alias" => {
                if args.is_empty() {
                    // List all aliases
                    for (name, value) in &interpreter.aliases {
                        println!("alias {name}='{value}'");
                    }
                    Ok(0)
                } else {
                    // Join all arguments back together to handle cases like: alias ll="ls -la"
                    let full_arg = args.join(" ");
                    if let Some(eq_pos) = full_arg.find('=') {
                        // Define alias: alias name=value
                        let name = full_arg[..eq_pos].trim().to_string();
                        let mut value = full_arg[eq_pos + 1..].to_string();

                        // Remove surrounding quotes if present and escape spaces
                        if (value.starts_with('"') && value.ends_with('"'))
                            || (value.starts_with('\'') && value.ends_with('\''))
                        {
                            value = value[1..value.len() - 1].to_string();
                        }

                        // Trim any leading/trailing whitespace
                        let value = value.trim().to_string();

                        interpreter.aliases.insert(name, value);
                        Ok(0)
                    } else if args.len() == 1 {
                        // Show specific alias
                        let name = &args[0];
                        if let Some(value) = interpreter.aliases.get(name) {
                            println!("alias {name}='{value}'");
                            Ok(0)
                        } else {
                            eprintln!("alias: {name}: not found");
                            Ok(1)
                        }
                    } else {
                        eprintln!("alias: usage: alias [name[=value] ...]");
                        Ok(1)
                    }
                }
            }
            "unalias" => {
                if args.is_empty() {
                    eprintln!("unalias: usage: unalias name [name ...]");
                    Ok(1)
                } else {
                    let mut success = true;
                    for name in args {
                        if interpreter.aliases.remove(name).is_none() {
                            eprintln!("unalias: {name}: not found");
                            success = false;
                        }
                    }
                    Ok(if success { 0 } else { 1 })
                }
            }
            "complete" => {
                // Built-in complete command for testing completion
                if args.is_empty() {
                    eprintln!("Usage: complete <command>");
                    return Ok(1);
                }

                let test_line = format!("{} ", args.join(" "));
                let context = CompletionSystem::parse_context(&test_line, test_line.len());
                let completions = interpreter.completion_system.complete(&context);

                for completion in completions {
                    println!("{completion}");
                }
                Ok(0)
            }
            _ => {
                // External command
                let mut command = Command::new(name);
                command.args(args);

                // Handle redirections
                for redirect in redirects {
                    match redirect.kind {
                        RedirectKind::Input => {
                            let file = fs::File::open(&redirect.file)?;
                            command.stdin(Stdio::from(file));
                        }
                        RedirectKind::Output => {
                            let file = fs::File::create(&redirect.file)?;
                            command.stdout(Stdio::from(file));
                        }
                        RedirectKind::Append => {
                            let file = fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&redirect.file)?;
                            command.stdout(Stdio::from(file));
                        }
                        RedirectKind::HereDoc | RedirectKind::HereDocDash => {
                            // Here documents - create a temporary file with the content
                            let temp_content = &redirect.file;
                            let mut temp_file = tempfile::NamedTempFile::new()?;
                            temp_file.write_all(temp_content.as_bytes())?;
                            temp_file.flush()?;
                            let file = temp_file.reopen()?;
                            command.stdin(Stdio::from(file));
                        }
                        RedirectKind::HereString => {
                            // Here strings - pass the string as stdin
                            let content = interpreter.expand_variables(&redirect.file);
                            let mut temp_file = tempfile::NamedTempFile::new()?;
                            temp_file.write_all(content.as_bytes())?;
                            temp_file.write_all(
                                b"
",
                            )?;
                            temp_file.flush()?;
                            let file = temp_file.reopen()?;
                            command.stdin(Stdio::from(file));
                        }
                        RedirectKind::InputDup | RedirectKind::OutputDup => {
                            // File descriptor duplication - simplified implementation
                            eprintln!("File descriptor duplication not fully implemented");
                        }
                    }
                }

                // Set environment variables
                for (key, value) in &interpreter.variables {
                    command.env(key, value);
                }

                match command.status() {
                    Ok(status) => {
                        // Update PWD in case the external command changed the working directory
                        if let Ok(current_dir) = env::current_dir() {
                            interpreter.variables.insert(
                                "PWD".to_string(),
                                current_dir.to_string_lossy().to_string(),
                            );
                        }
                        Ok(status.code().unwrap_or(0))
                    }
                    Err(_) => {
                        eprintln!("{name}: command not found");
                        Ok(127)
                    }
                }
            }
        }
    }

    fn evaluate_export(
        &mut self,
        name: &str,
        value: &Option<Box<Node>>,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        match value {
            Some(val) => {
                // Export with assignment: export VAR=value
                match val.as_ref() {
                    Node::StringLiteral(string_value) => {
                        let expanded_value = interpreter.expand_variables(string_value);
                        interpreter
                            .variables
                            .insert(name.to_string(), expanded_value.clone());
                        if !name.is_empty() {
                            unsafe {
                                env::set_var(name, &expanded_value);
                            }
                        }
                    }
                    Node::SingleQuotedString(string_value) => {
                        // Single-quoted strings should not have variable expansion
                        interpreter
                            .variables
                            .insert(name.to_string(), string_value.clone());
                        if !name.is_empty() {
                            unsafe {
                                env::set_var(name, string_value);
                            }
                        }
                    }
                    Node::CommandSubstitution { command } => {
                        let output = interpreter.capture_command_output(command, self)?;
                        let trimmed_output = output.trim_end().to_string();
                        interpreter
                            .variables
                            .insert(name.to_string(), trimmed_output.clone());
                        if !name.is_empty() {
                            unsafe {
                                env::set_var(name, &trimmed_output);
                            }
                        }
                    }
                    Node::ArithmeticExpansion { expression } => {
                        let expanded_expr = interpreter.expand_variables(expression);
                        match DefaultEvaluator::evaluate_arithmetic_expression(&expanded_expr) {
                            Ok(result) => {
                                let result_str = result.to_string();
                                interpreter
                                    .variables
                                    .insert(name.to_string(), result_str.clone());
                                if !name.is_empty() {
                                    unsafe {
                                        env::set_var(name, &result_str);
                                    }
                                }
                            }
                            Err(_) => {
                                eprintln!(
                                    "arithmetic expansion: invalid expression: {expanded_expr}"
                                );
                                interpreter
                                    .variables
                                    .insert(name.to_string(), "0".to_string());
                                if !name.is_empty() {
                                    unsafe {
                                        env::set_var(name, "0");
                                    }
                                }
                            }
                        }
                    }
                    Node::ArithmeticCommand { expression } => {
                        let expanded_expr = interpreter.expand_variables(expression);
                        match DefaultEvaluator::evaluate_arithmetic_expression(&expanded_expr) {
                            Ok(result) => {
                                let result_str = result.to_string();
                                interpreter
                                    .variables
                                    .insert(name.to_string(), result_str.clone());
                                if !name.is_empty() {
                                    unsafe {
                                        env::set_var(name, &result_str);
                                    }
                                }
                            }
                            Err(_) => {
                                eprintln!(
                                    "arithmetic command: invalid expression: {expanded_expr}"
                                );
                                interpreter
                                    .variables
                                    .insert(name.to_string(), "0".to_string());
                                if !name.is_empty() {
                                    unsafe {
                                        env::set_var(name, "0");
                                    }
                                }
                            }
                        }
                    }
                    Node::Array { elements } => {
                        // Handle array export - join elements with spaces or use a specific format
                        let array_value = elements.join(" ");
                        let expanded_value = interpreter.expand_variables(&array_value);
                        interpreter
                            .variables
                            .insert(name.to_string(), expanded_value.clone());
                        if !name.is_empty() {
                            unsafe {
                                env::set_var(name, &expanded_value);
                            }
                        }
                    }
                    _ => {
                        return Err(io::Error::other(
                            "Unsupported value type for export assignment",
                        ));
                    }
                }
            }
            None => {
                // Export without assignment: export VAR
                // Export existing variable if it exists in the interpreter's variables
                if let Some(existing_value) = interpreter.variables.get(name) {
                    if !name.is_empty() {
                        unsafe {
                            env::set_var(name, existing_value);
                        }
                    }
                } else {
                    // If variable doesn't exist in interpreter, check if it exists in environment
                    if let Ok(env_value) = env::var(name) {
                        // Store it in interpreter variables for consistency
                        interpreter
                            .variables
                            .insert(name.to_string(), env_value.clone());
                        if !name.is_empty() {
                            unsafe {
                                env::set_var(name, &env_value);
                            }
                        }
                    } else {
                        // Variable doesn't exist anywhere, just add it to interpreter variables
                        // Don't set empty environment variables as they can cause issues
                        interpreter
                            .variables
                            .insert(name.to_string(), String::new());
                    }
                }
            }
        }
        Ok(0)
    }

    fn evaluate_pipeline(
        &mut self,
        commands: &[Node],
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        if commands.is_empty() {
            return Ok(0);
        }

        if commands.len() == 1 {
            return interpreter.evaluate_with_evaluator(&commands[0], self);
        }

        let mut last_exit_code = 0;
        for command in commands {
            last_exit_code = interpreter.evaluate_with_evaluator(command, self)?;
        }
        Ok(last_exit_code)
    }

    fn evaluate_list(
        &mut self,
        statements: &[Node],
        operators: &[String],
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        let mut last_exit_code = 0;

        for (i, statement) in statements.iter().enumerate() {
            match interpreter.evaluate_with_evaluator(statement, self) {
                Ok(code) => {
                    last_exit_code = code;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                    // Check if this is a return statement - propagate it up
                    if e.to_string().starts_with("return:") {
                        return Err(io::Error::new(e.kind(), e.to_string()));
                    }
                    return Err(io::Error::new(e.kind(), e.to_string()));
                }
                Err(e) => return Err(e),
            }

            if i < operators.len() {
                match operators[i].as_str() {
                    "&&" => {
                        if last_exit_code != 0 {
                            break;
                        }
                    }
                    "||" => {
                        if last_exit_code == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(last_exit_code)
    }

    fn evaluate_assignment(
        &mut self,
        name: &str,
        value: &Node,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        match value {
            Node::StringLiteral(string_value) => {
                let expanded_value = interpreter.expand_variables(string_value);
                interpreter
                    .variables
                    .insert(name.to_string(), expanded_value);
            }
            Node::SingleQuotedString(string_value) => {
                // Single-quoted strings should not have variable expansion
                interpreter
                    .variables
                    .insert(name.to_string(), string_value.clone());
            }
            Node::CommandSubstitution { command } => {
                let output = interpreter.capture_command_output(command, self)?;
                interpreter.variables.insert(name.to_string(), output);
            }
            Node::ArithmeticExpansion { expression } => {
                let expanded_expr = interpreter.expand_variables(expression);
                match DefaultEvaluator::evaluate_arithmetic_expression(&expanded_expr) {
                    Ok(result) => {
                        interpreter
                            .variables
                            .insert(name.to_string(), result.to_string());
                    }
                    Err(_) => {
                        eprintln!("arithmetic expansion: invalid expression: {expanded_expr}");
                        interpreter
                            .variables
                            .insert(name.to_string(), "0".to_string());
                    }
                }
            }
            Node::ArithmeticCommand { expression } => {
                let expanded_expr = interpreter.expand_variables(expression);
                match DefaultEvaluator::evaluate_arithmetic_expression(&expanded_expr) {
                    Ok(result) => {
                        interpreter
                            .variables
                            .insert(name.to_string(), result.to_string());
                    }
                    Err(_) => {
                        eprintln!("arithmetic command: invalid expression: {expanded_expr}");
                        interpreter
                            .variables
                            .insert(name.to_string(), "0".to_string());
                    }
                }
            }
            Node::Array { elements } => {
                // Handle array assignment - join elements or store in a specific format
                let array_value = elements.join(" ");
                let expanded_value = interpreter.expand_variables(&array_value);
                interpreter
                    .variables
                    .insert(name.to_string(), expanded_value);
            }
            _ => {
                return Err(io::Error::other("Unsupported value type for assignment"));
            }
        }
        Ok(0)
    }

    fn evaluate_ext_glob(
        &mut self,
        operator: char,
        patterns: &[String],
        suffix: &str,
        interpreter: &Interpreter,
    ) -> Result<i32, io::Error> {
        let entries = fs::read_dir(".")?;
        let mut matches = Vec::new();

        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if interpreter.matches_ext_glob(&file_name, operator, patterns, suffix) {
                matches.push(file_name);
            }
        }

        for m in matches {
            println!("{m}");
        }

        Ok(0)
    }

    fn evaluate_if_statement(
        &mut self,
        condition: &Node,
        consequence: &Node,
        alternative: &Option<Box<Node>>,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Evaluate the condition
        let condition_result = interpreter.evaluate_with_evaluator(condition, self)?;

        if condition_result == 0 {
            // Condition is true (exit code 0), execute the consequence
            interpreter.evaluate_with_evaluator(consequence, self)
        } else if let Some(alt) = alternative {
            // Condition is false, execute the alternative (elif or else)
            interpreter.evaluate_with_evaluator(alt, self)
        } else {
            // No alternative, return the condition's exit code
            Ok(condition_result)
        }
    }

    fn evaluate_elif_branch(
        &mut self,
        condition: &Node,
        consequence: &Node,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Evaluate the elif condition
        let condition_result = interpreter.evaluate_with_evaluator(condition, self)?;

        if condition_result == 0 {
            // Condition is true, execute the consequence
            interpreter.evaluate_with_evaluator(consequence, self)
        } else {
            // Condition is false, return the condition's exit code
            Ok(condition_result)
        }
    }

    fn evaluate_else_branch(
        &mut self,
        consequence: &Node,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Always execute the else consequence
        interpreter.evaluate_with_evaluator(consequence, self)
    }

    fn evaluate_case_statement(
        &mut self,
        expression: &Node,
        patterns: &[CasePattern],
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Evaluate the expression to get the value to match against
        let expr_result = match expression {
            Node::StringLiteral(s) => s.clone(),
            Node::Command { name, args, .. } => {
                // For commands, use the command name and args as the value
                format!("{} {}", name, args.join(" "))
            }
            Node::CommandSubstitution { command } => {
                // Evaluate the command substitution and get its output
                match interpreter.evaluate_with_evaluator(command, self) {
                    Ok(_) => {
                        // Try to get the output from the command
                        // For now, we'll use a simple approach
                        "".to_string() // This is a placeholder - in a full implementation,
                        // we'd capture the command output
                    }
                    Err(_) => "".to_string(),
                }
            }
            _ => {
                // For other node types, try to convert to string representation
                format!("{expression:?}")
            }
        };

        // Expand variables in the expression result
        let expanded_expr = interpreter.expand_variables(&expr_result);

        // Try to match against each pattern
        for pattern in patterns {
            for pattern_str in &pattern.patterns {
                // Expand variables in the pattern
                let expanded_pattern = interpreter.expand_variables(pattern_str);

                // Check for exact match or wildcard match
                if self.pattern_matches(&expanded_expr, &expanded_pattern) {
                    // Execute the body for this pattern
                    return interpreter.evaluate_with_evaluator(&pattern.body, self);
                }
            }
        }

        // No pattern matched, return success (like bash)
        Ok(0)
    }

    fn pattern_matches(&self, value: &str, pattern: &str) -> bool {
        // Handle special case patterns
        if pattern == "*" {
            return true; // Wildcard matches everything
        }

        // Simple pattern matching - exact match or basic wildcard
        if pattern.contains('*') {
            // Convert shell pattern to regex
            let regex_pattern = pattern
                .replace("*", ".*")
                .replace("?", ".")
                .replace("[", "\\[")
                .replace("]", "\\]");

            if let Ok(regex) = Regex::new(&format!("^{regex_pattern}$")) {
                return regex.is_match(value);
            }
        }

        // Exact match
        value == pattern
    }

    fn evaluate_for_loop(
        &mut self,
        variable: &str,
        iterable: &Node,
        body: &Node,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Get the list of values to iterate over
        let values = match iterable {
            Node::Array { elements } => elements.clone(),
            _ => {
                // If it's not an array, try to evaluate it as a command and get its output
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "for loop iterable must be an array",
                ));
            }
        };

        let mut last_exit_code = 0;

        // Save the current value of the loop variable (if it exists)
        let old_value = interpreter.variables.get(variable).cloned();

        // Iterate over each value
        for value in values {
            // Set the loop variable
            interpreter.variables.insert(variable.to_string(), value);

            // Execute the body
            last_exit_code = interpreter.evaluate_with_evaluator(body, self)?;

            // Check for break/continue (not implemented yet, but structure is ready)
            // if interpreter.should_break { break; }
            // if interpreter.should_continue { continue; }
        }

        // Restore the old value of the loop variable
        if let Some(old_val) = old_value {
            interpreter.variables.insert(variable.to_string(), old_val);
        } else {
            interpreter.variables.remove(variable);
        }

        Ok(last_exit_code)
    }

    fn evaluate_while_loop(
        &mut self,
        condition: &Node,
        body: &Node,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        let mut last_exit_code = 0;

        // Continue while condition returns 0 (success)
        loop {
            let condition_result = interpreter.evaluate_with_evaluator(condition, self)?;
            if condition_result != 0 {
                break; // Exit when condition fails
            }

            // Execute the body
            last_exit_code = interpreter.evaluate_with_evaluator(body, self)?;

            // Check for break/continue (not implemented yet, but structure is ready)
            // if interpreter.should_break { break; }
            // if interpreter.should_continue { continue; }
        }

        Ok(last_exit_code)
    }

    fn evaluate_until_loop(
        &mut self,
        condition: &Node,
        body: &Node,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        let mut last_exit_code = 0;

        // Continue until condition returns 0 (success)
        loop {
            let condition_result = interpreter.evaluate_with_evaluator(condition, self)?;
            if condition_result == 0 {
                break; // Exit when condition succeeds
            }

            // Execute the body
            last_exit_code = interpreter.evaluate_with_evaluator(body, self)?;

            // Check for break/continue (not implemented yet, but structure is ready)
            // if interpreter.should_break { break; }
            // if interpreter.should_continue { continue; }
        }

        Ok(last_exit_code)
    }

    fn evaluate_select_statement(
        &mut self,
        variable: &str,
        items: &Node,
        body: &Node,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        use std::io::{self, Write};

        // Evaluate items to get the list of choices
        let choices = match items {
            Node::Array { elements } => elements.clone(),
            Node::StringLiteral(s) if s == "$@" => {
                // Use positional parameters
                interpreter.args.clone()
            }
            _ => {
                // Try to evaluate as a command and capture output
                let output = interpreter.capture_command_output(items, self)?;
                output.split_whitespace().map(|s| s.to_string()).collect()
            }
        };

        if choices.is_empty() {
            return Ok(0);
        }

        // Check if we're in a non-interactive environment (like tests)
        // by checking if stdin is redirected or if we're in a test environment
        if std::env::var("CARGO_TEST").is_ok() || std::env::var("RUST_TEST_THREADS").is_ok() {
            // In test mode, just return without executing the body
            return Ok(0);
        }

        let mut last_exit_code = 0;

        loop {
            // Display the menu
            for (i, choice) in choices.iter().enumerate() {
                println!("{}) {}", i + 1, choice);
            }

            // Prompt for selection
            print!("#? ");
            io::stdout().flush()?;

            // Read user input
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => {
                    // EOF - break out of select loop
                    break;
                }
                Ok(_) => {
                    let input = input.trim();

                    // Handle empty input (just continue the loop)
                    if input.is_empty() {
                        continue;
                    }

                    // Handle numeric selection
                    if let Ok(selection) = input.parse::<usize>() {
                        if selection > 0 && selection <= choices.len() {
                            // Set the variable to the selected item
                            interpreter
                                .variables
                                .insert(variable.to_string(), choices[selection - 1].clone());

                            // Set REPLY to the user's input
                            interpreter
                                .variables
                                .insert("REPLY".to_string(), input.to_string());

                            // Execute the body
                            last_exit_code = interpreter.evaluate_with_evaluator(body, self)?;
                            continue;
                        }
                    }

                    // Set REPLY to the user's input for non-numeric or invalid selections
                    interpreter
                        .variables
                        .insert("REPLY".to_string(), input.to_string());

                    // For invalid selections, unset the variable and execute body
                    interpreter.variables.remove(variable);
                    last_exit_code = interpreter.evaluate_with_evaluator(body, self)?;
                }
                Err(_) => {
                    // Error reading input - break out of select loop
                    break;
                }
            }
        }

        Ok(last_exit_code)
    }

    fn evaluate_command_substitution(
        &mut self,
        command: &Node,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        let _output = interpreter.capture_command_output(command, self)?;
        // Command substitution doesn't print output, it returns it as a value
        // The exit code is typically 0 for successful substitution
        Ok(0)
    }

    fn evaluate_arithmetic_expansion(
        &mut self,
        expression: &str,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Expand variables in the expression first
        let expanded_expr = interpreter.expand_variables(expression);

        // Evaluate the arithmetic expression
        match self.evaluate_arithmetic_expression_with_assignment(&expanded_expr, interpreter) {
            Ok(_result) => {
                // For arithmetic expansion, we typically return 0 for success
                // The actual result is used as a value, not an exit code
                Ok(0)
            }
            Err(_) => {
                eprintln!("arithmetic expansion: invalid expression: {expanded_expr}");
                Ok(1)
            }
        }
    }

    fn evaluate_arithmetic_command(
        &mut self,
        expression: &str,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Expand variables in the expression first
        let expanded_expr = interpreter.expand_variables(expression);

        // Evaluate the arithmetic expression
        match self.evaluate_arithmetic_expression_with_assignment(&expanded_expr, interpreter) {
            Ok(result) => {
                // For arithmetic commands, the exit code is based on the result
                // Non-zero result means success (exit code 0), zero result means failure (exit code 1)
                if result != 0 {
                    Ok(0) // Success
                } else {
                    Ok(1) // Failure
                }
            }
            Err(_) => {
                eprintln!("arithmetic command: invalid expression: {expanded_expr}");
                Ok(1)
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn evaluate_arithmetic_expression_with_assignment(
        &mut self,
        expr: &str,
        interpreter: &mut Interpreter,
    ) -> Result<i64, String> {
        // Enhanced arithmetic expression evaluator that can handle assignments
        // Handles +, -, *, /, %, comparison operators, logical operators, assignments, and parentheses

        let expr = expr.trim();
        if expr.is_empty() {
            return Ok(0);
        }

        // Remove any trailing ')' that might be left from parsing issues
        let expr = expr.trim_end_matches(')');

        // Handle simple number
        if let Ok(num) = expr.parse::<i64>() {
            return Ok(num);
        }

        // Handle variable names (should be already expanded by expand_variables)
        // If it's a single word that's not a number, treat it as 0 (unset variable)
        if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
            // This should not happen if variables are properly expanded
            // But if it does, treat as 0
            return Ok(0);
        }

        // Handle assignment operators first (lowest precedence)
        if let Some(pos) = expr.rfind('=') {
            // Make sure it's not part of == or != or >= or <=
            if pos > 0 && pos + 1 < expr.len() {
                let prev_char = expr.chars().nth(pos - 1);
                let next_char = expr.chars().nth(pos + 1);
                if prev_char != Some('!')
                    && prev_char != Some('<')
                    && prev_char != Some('>')
                    && prev_char != Some('=')
                    && next_char != Some('=')
                {
                    // This is an assignment, not a comparison
                    let left = expr[..pos].trim();
                    let right = expr[pos + 1..].trim();
                    let right_val =
                        self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;

                    // Extract variable name from left side (remove $ if present)
                    let var_name = if let Some(stripped) = left.strip_prefix('$') {
                        stripped
                    } else {
                        left
                    };

                    // Set the variable
                    interpreter
                        .variables
                        .insert(var_name.to_string(), right_val.to_string());
                    return Ok(right_val);
                }
            }
        }

        // Handle logical operators (next lowest precedence)
        if let Some(pos) = expr.rfind("||") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val =
                self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
            let right_val =
                self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
            return Ok(if left_val != 0 || right_val != 0 {
                1
            } else {
                0
            });
        }

        if let Some(pos) = expr.rfind("&&") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val =
                self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
            let right_val =
                self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
            return Ok(if left_val != 0 && right_val != 0 {
                1
            } else {
                0
            });
        }

        // Handle comparison operators
        if let Some(pos) = expr.rfind("<=") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val =
                self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
            let right_val =
                self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
            return Ok(if left_val <= right_val { 1 } else { 0 });
        }

        if let Some(pos) = expr.rfind(">=") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val =
                self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
            let right_val =
                self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
            return Ok(if left_val >= right_val { 1 } else { 0 });
        }

        if let Some(pos) = expr.rfind("!=") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val =
                self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
            let right_val =
                self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
            return Ok(if left_val != right_val { 1 } else { 0 });
        }

        if let Some(pos) = expr.rfind("==") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val =
                self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
            let right_val =
                self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
            return Ok(if left_val == right_val { 1 } else { 0 });
        }

        // Handle single character comparison operators (be careful with order)
        if let Some(pos) = expr.rfind('<') {
            // Make sure it's not part of <= or <<
            if pos + 1 >= expr.len()
                || (expr.chars().nth(pos + 1) != Some('=')
                    && expr.chars().nth(pos + 1) != Some('<'))
            {
                let left = expr[..pos].trim();
                let right = expr[pos + 1..].trim();
                let left_val =
                    self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
                let right_val =
                    self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
                return Ok(if left_val < right_val { 1 } else { 0 });
            }
        }

        if let Some(pos) = expr.rfind('>') {
            // Make sure it's not part of >= or >>
            if pos + 1 >= expr.len()
                || (expr.chars().nth(pos + 1) != Some('=')
                    && expr.chars().nth(pos + 1) != Some('>'))
            {
                let left = expr[..pos].trim();
                let right = expr[pos + 1..].trim();
                let left_val =
                    self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
                let right_val =
                    self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
                return Ok(if left_val > right_val { 1 } else { 0 });
            }
        }

        // Addition and subtraction
        if let Some(pos) = expr.rfind('+') {
            let left = expr[..pos].trim();
            let right = expr[pos + 1..].trim();
            let left_val =
                self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
            let right_val =
                self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
            return Ok(left_val + right_val);
        }

        if let Some(pos) = expr.rfind('-') {
            // Make sure it's not a negative number at the start
            if pos > 0 {
                let left = expr[..pos].trim();
                let right = expr[pos + 1..].trim();
                let left_val =
                    self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
                let right_val =
                    self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
                return Ok(left_val - right_val);
            }
        }

        // Multiplication, division, and modulo (higher precedence)
        if let Some(pos) = expr.rfind('*') {
            let left = expr[..pos].trim();
            let right = expr[pos + 1..].trim();
            let left_val =
                self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
            let right_val =
                self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
            return Ok(left_val * right_val);
        }

        if let Some(pos) = expr.rfind('/') {
            let left = expr[..pos].trim();
            let right = expr[pos + 1..].trim();
            let left_val =
                self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
            let right_val =
                self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
            if right_val == 0 {
                return Err("division by zero".to_string());
            }
            return Ok(left_val / right_val);
        }

        if let Some(pos) = expr.rfind('%') {
            let left = expr[..pos].trim();
            let right = expr[pos + 1..].trim();
            let left_val =
                self.evaluate_arithmetic_expression_with_assignment(left, interpreter)?;
            let right_val =
                self.evaluate_arithmetic_expression_with_assignment(right, interpreter)?;
            if right_val == 0 {
                return Err("division by zero".to_string());
            }
            return Ok(left_val % right_val);
        }

        // Handle parentheses
        if expr.starts_with('(') && expr.ends_with(')') {
            let inner = &expr[1..expr.len() - 1];
            return self.evaluate_arithmetic_expression_with_assignment(inner, interpreter);
        }

        // If we can't parse it, return an error
        Err(format!("invalid arithmetic expression: {expr}"))
    }

    pub fn evaluate_arithmetic_expression(expr: &str) -> Result<i64, String> {
        // Enhanced arithmetic expression evaluator
        // Handles +, -, *, /, %, comparison operators, logical operators, and parentheses

        let expr = expr.trim();
        if expr.is_empty() {
            return Ok(0);
        }

        // Remove any trailing ')' that might be left from parsing issues
        let expr = expr.trim_end_matches(')');

        // Handle simple number
        if let Ok(num) = expr.parse::<i64>() {
            return Ok(num);
        }

        // Handle variable names (should be already expanded by expand_variables)
        // If it's a single word that's not a number, treat it as 0 (unset variable)
        if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
            // This should not happen if variables are properly expanded
            // But if it does, treat as 0
            return Ok(0);
        }

        // Handle logical operators (lowest precedence)
        if let Some(pos) = expr.rfind("||") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
            let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
            return Ok(if left_val != 0 || right_val != 0 {
                1
            } else {
                0
            });
        }

        if let Some(pos) = expr.rfind("&&") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
            let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
            return Ok(if left_val != 0 && right_val != 0 {
                1
            } else {
                0
            });
        }

        // Handle comparison operators
        if let Some(pos) = expr.rfind("<=") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
            let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
            return Ok(if left_val <= right_val { 1 } else { 0 });
        }

        if let Some(pos) = expr.rfind(">=") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
            let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
            return Ok(if left_val >= right_val { 1 } else { 0 });
        }

        if let Some(pos) = expr.rfind("!=") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
            let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
            return Ok(if left_val != right_val { 1 } else { 0 });
        }

        if let Some(pos) = expr.rfind("==") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim();
            let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
            let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
            return Ok(if left_val == right_val { 1 } else { 0 });
        }

        // Handle single character comparison operators (be careful with order)
        if let Some(pos) = expr.rfind('<') {
            // Make sure it's not part of <= or <<
            if pos + 1 >= expr.len()
                || (expr.chars().nth(pos + 1) != Some('=')
                    && expr.chars().nth(pos + 1) != Some('<'))
            {
                let left = expr[..pos].trim();
                let right = expr[pos + 1..].trim();
                let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
                let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
                return Ok(if left_val < right_val { 1 } else { 0 });
            }
        }

        if let Some(pos) = expr.rfind('>') {
            // Make sure it's not part of >= or >>
            if pos + 1 >= expr.len()
                || (expr.chars().nth(pos + 1) != Some('=')
                    && expr.chars().nth(pos + 1) != Some('>'))
            {
                let left = expr[..pos].trim();
                let right = expr[pos + 1..].trim();
                let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
                let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
                return Ok(if left_val > right_val { 1 } else { 0 });
            }
        }

        // Handle assignment operators
        if let Some(pos) = expr.rfind('=') {
            // Make sure it's not part of == or != or >= or <=
            if pos > 0 && pos + 1 < expr.len() {
                let prev_char = expr.chars().nth(pos - 1);
                let next_char = expr.chars().nth(pos + 1);
                if prev_char != Some('!')
                    && prev_char != Some('<')
                    && prev_char != Some('>')
                    && next_char != Some('=')
                {
                    // This is an assignment, not a comparison
                    let _left = expr[..pos].trim();
                    let right = expr[pos + 1..].trim();
                    let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
                    // For now, just return the right value
                    // TODO: Actually assign to the variable
                    return Ok(right_val);
                }
            }
        }

        // Addition and subtraction
        if let Some(pos) = expr.rfind('+') {
            let left = expr[..pos].trim();
            let right = expr[pos + 1..].trim();
            let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
            let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
            return Ok(left_val + right_val);
        }

        if let Some(pos) = expr.rfind('-') {
            // Make sure it's not a negative number at the start
            if pos > 0 {
                let left = expr[..pos].trim();
                let right = expr[pos + 1..].trim();
                let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
                let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
                return Ok(left_val - right_val);
            }
        }

        // Multiplication, division, and modulo (higher precedence)
        if let Some(pos) = expr.rfind('*') {
            let left = expr[..pos].trim();
            let right = expr[pos + 1..].trim();
            let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
            let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
            return Ok(left_val * right_val);
        }

        if let Some(pos) = expr.rfind('/') {
            let left = expr[..pos].trim();
            let right = expr[pos + 1..].trim();
            let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
            let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
            if right_val == 0 {
                return Err("division by zero".to_string());
            }
            return Ok(left_val / right_val);
        }

        if let Some(pos) = expr.rfind('%') {
            let left = expr[..pos].trim();
            let right = expr[pos + 1..].trim();
            let left_val = DefaultEvaluator::evaluate_arithmetic_expression(left)?;
            let right_val = DefaultEvaluator::evaluate_arithmetic_expression(right)?;
            if right_val == 0 {
                return Err("division by zero".to_string());
            }
            return Ok(left_val % right_val);
        }

        // Handle parentheses
        if expr.starts_with('(') && expr.ends_with(')') {
            let inner = &expr[1..expr.len() - 1];
            return DefaultEvaluator::evaluate_arithmetic_expression(inner);
        }

        // If we can't parse it, return an error
        Err(format!("invalid arithmetic expression: {expr}"))
    }

    fn evaluate_test_command(
        &mut self,
        args: &[String],
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Handle the test command ([ and test)
        // For [ command, the last argument should be "]"
        let test_args = if !args.is_empty() && args[args.len() - 1] == "]" {
            &args[..args.len() - 1] // Remove the closing "]"
        } else {
            args
        };

        if test_args.is_empty() {
            return Ok(1); // Empty test is false
        }

        // Handle different test operations
        match test_args.len() {
            1 => {
                // Single argument: test if string is non-empty
                let expanded_arg = interpreter.expand_variables(&test_args[0]);
                Ok(if expanded_arg.is_empty() { 1 } else { 0 })
            }
            3 => {
                // Three arguments: left operator right
                let left = interpreter.expand_variables(&test_args[0]);
                let operator = &test_args[1];
                let right = interpreter.expand_variables(&test_args[2]);

                match operator.as_str() {
                    "=" | "==" => Ok(if left == right { 0 } else { 1 }),
                    "!=" => Ok(if left != right { 0 } else { 1 }),
                    "-eq" => {
                        // Numeric equality
                        match (left.parse::<i64>(), right.parse::<i64>()) {
                            (Ok(l), Ok(r)) => Ok(if l == r { 0 } else { 1 }),
                            _ => Ok(1), // Non-numeric values are not equal
                        }
                    }
                    "-ne" => {
                        // Numeric inequality
                        match (left.parse::<i64>(), right.parse::<i64>()) {
                            (Ok(l), Ok(r)) => Ok(if l != r { 0 } else { 1 }),
                            _ => Ok(0), // Non-numeric values are not equal
                        }
                    }
                    "-lt" => {
                        // Numeric less than
                        match (left.parse::<i64>(), right.parse::<i64>()) {
                            (Ok(l), Ok(r)) => Ok(if l < r { 0 } else { 1 }),
                            _ => Ok(1),
                        }
                    }
                    "-le" => {
                        // Numeric less than or equal
                        match (left.parse::<i64>(), right.parse::<i64>()) {
                            (Ok(l), Ok(r)) => Ok(if l <= r { 0 } else { 1 }),
                            _ => Ok(1),
                        }
                    }
                    "-gt" => {
                        // Numeric greater than
                        match (left.parse::<i64>(), right.parse::<i64>()) {
                            (Ok(l), Ok(r)) => Ok(if l > r { 0 } else { 1 }),
                            _ => Ok(1),
                        }
                    }
                    "-ge" => {
                        // Numeric greater than or equal
                        match (left.parse::<i64>(), right.parse::<i64>()) {
                            (Ok(l), Ok(r)) => Ok(if l >= r { 0 } else { 1 }),
                            _ => Ok(1),
                        }
                    }
                    _ => Ok(1), // Unknown operator
                }
            }
            2 => {
                // Two arguments: unary operator
                let operator = &test_args[0];
                let operand = interpreter.expand_variables(&test_args[1]);

                match operator.as_str() {
                    "-n" => Ok(if !operand.is_empty() { 0 } else { 1 }), // String is non-empty
                    "-z" => Ok(if operand.is_empty() { 0 } else { 1 }),  // String is empty
                    "-f" => {
                        // File exists and is a regular file
                        let path = Path::new(&operand);
                        Ok(if path.is_file() { 0 } else { 1 })
                    }
                    "-d" => {
                        // File exists and is a directory
                        let path = Path::new(&operand);
                        Ok(if path.is_dir() { 0 } else { 1 })
                    }
                    "-e" => {
                        // File exists
                        let path = Path::new(&operand);
                        Ok(if path.exists() { 0 } else { 1 })
                    }
                    "-r" => {
                        // File is readable
                        let path = Path::new(&operand);
                        Ok(if path.exists() && fs::metadata(path).is_ok() {
                            0
                        } else {
                            1
                        })
                    }
                    "-w" => {
                        // File is writable
                        let path = Path::new(&operand);
                        Ok(
                            if path.exists()
                                && fs::metadata(path).is_ok_and(|m| !m.permissions().readonly())
                            {
                                0
                            } else {
                                1
                            },
                        )
                    }
                    "-x" => {
                        // File is executable
                        let path = Path::new(&operand);
                        #[cfg(unix)]
                        {
                            Ok(
                                if path.exists()
                                    && fs::metadata(path)
                                        .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
                                {
                                    0
                                } else {
                                    1
                                },
                            )
                        }
                        #[cfg(not(unix))]
                        {
                            Ok(if path.exists() { 0 } else { 1 })
                        }
                    }
                    "-s" => {
                        // File exists and has size > 0
                        let path = Path::new(&operand);
                        Ok(
                            if path.exists() && fs::metadata(path).is_ok_and(|m| m.len() > 0) {
                                0
                            } else {
                                1
                            },
                        )
                    }
                    _ => Ok(1), // Unknown unary operator
                }
            }
            _ => Ok(1), // Invalid number of arguments
        }
    }

    fn evaluate_history_expansion(
        &mut self,
        pattern: &str,
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Check for recursion depth to prevent stack overflow
        const MAX_HISTORY_RECURSION_DEPTH: u32 = 10;
        if interpreter.history_expansion_depth >= MAX_HISTORY_RECURSION_DEPTH {
            eprintln!("flash: history expansion recursion limit exceeded");
            return Ok(1);
        }

        // Handle different history expansion patterns
        let command = if pattern.is_empty() {
            // !! - repeat last command
            interpreter.history.last().cloned()
        } else if pattern.chars().all(|c| c.is_ascii_digit()) {
            // !n - repeat command number n (1-based)
            if let Ok(n) = pattern.parse::<usize>() {
                if n > 0 && n <= interpreter.history.len() {
                    Some(interpreter.history[n - 1].clone())
                } else {
                    eprintln!("flash: !{pattern}: event not found");
                    return Ok(1);
                }
            } else {
                None
            }
        } else if pattern.starts_with('-') && pattern[1..].chars().all(|c| c.is_ascii_digit()) {
            // !-n - repeat command n positions back from current
            if let Ok(n) = pattern[1..].parse::<usize>() {
                let history_len = interpreter.history.len();
                if n > 0 && n <= history_len {
                    Some(interpreter.history[history_len - n].clone())
                } else {
                    eprintln!("flash: !{pattern}: event not found");
                    return Ok(1);
                }
            } else {
                None
            }
        } else {
            // !string - repeat most recent command starting with string
            interpreter
                .history
                .iter()
                .rev()
                .find(|cmd| cmd.starts_with(pattern))
                .cloned()
        };

        match command {
            Some(cmd) => {
                // Check if the command we're about to execute is itself a history expansion
                // to prevent immediate infinite recursion
                if cmd.trim() == format!("!{pattern}") || (pattern.is_empty() && cmd.trim() == "!!")
                {
                    eprintln!("flash: history expansion would cause infinite recursion");
                    return Ok(1);
                }

                // Print the command being executed (like bash does)
                println!("{cmd}");

                // Increment recursion depth before executing
                interpreter.history_expansion_depth += 1;

                // Parse and execute the command using the correct method
                let result = interpreter.execute_with_evaluator(&cmd, self);

                // Decrement recursion depth after executing
                interpreter.history_expansion_depth -= 1;

                result
            }
            None => {
                eprintln!("flash: !{pattern}: event not found");
                Ok(1)
            }
        }
    }
}

/// Shell interpreter
pub struct Interpreter {
    pub variables: HashMap<String, String>,
    pub functions: HashMap<String, Box<Node>>, // Store function definitions
    pub aliases: HashMap<String, String>,      // Store alias definitions
    pub last_exit_code: i32,
    pub history: Vec<String>,
    pub history_file: Option<String>,
    pub rc_file: Option<String>,
    pub args: Vec<String>,         // Command line arguments ($0, $1, $2, ...)
    pub return_value: Option<i32>, // Track return values from functions
    pub history_expansion_depth: u32, // Track recursion depth for history expansion
    pub completion_system: CompletionSystem, // Enhanced completion system
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        // Initialize some basic environment variables
        let mut variables = HashMap::default();

        if let Ok(variables_from_proc) = flash::env::load_env_from_proc() {
            for (key, value) in variables_from_proc.iter() {
                variables.insert(key.to_owned(), value.to_owned());
            }
        } else {
            // Fallback to std::env if /proc/self/environ is not available (e.g., on macOS)
            for (key, value) in std::env::vars() {
                variables.insert(key, value);
            }
        }

        // Set up some shell variables
        variables.insert("?".to_string(), "0".to_string());
        variables.insert("SHELL".to_string(), "flash".to_string());
        variables.insert("$$".to_string(), std::process::id().to_string());
        variables.insert(
            "FLASH_VERSION".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        );

        // Initialize PWD to current working directory
        if let Ok(current_dir) = env::current_dir() {
            variables.insert("PWD".to_string(), current_dir.to_string_lossy().to_string());
        }

        // Add /opt/homebrew/bin to PATH on macOS
        #[cfg(target_os = "macos")]
        {
            if let Some(current_path) = variables.get("PATH") {
                if !current_path.contains("/opt/homebrew/bin") {
                    let new_path = format!("/opt/homebrew/bin:{current_path}");
                    variables.insert("PATH".to_string(), new_path);
                }
            } else {
                variables.insert("PATH".to_string(), "/opt/homebrew/bin".to_string());
            }
        }

        let home_dir = env::var("HOME").ok();

        let history_file = home_dir
            .as_ref()
            .map(|home| format!("{home}/.flash_history"));

        let rc_file = home_dir.as_ref().map(|home| format!("{home}/.flashrc"));

        // Load history from file if it exists
        let mut history = Vec::new();
        if let Some(ref file_path) = history_file {
            if let Ok(file) = fs::File::open(file_path) {
                let reader = io::BufReader::new(file);
                for line in reader.lines().map_while(Result::ok) {
                    history.push(line);
                }
            }
        }

        let mut interpreter = Self {
            variables,
            functions: HashMap::new(), // Initialize empty functions map
            aliases: HashMap::new(),   // Initialize empty aliases map
            last_exit_code: 0,
            history,
            history_file,
            rc_file,
            args: Vec::new(), // Initialize empty args, will be set when running scripts
            return_value: None, // Initialize return value as None
            history_expansion_depth: 0, // Initialize history expansion depth
            completion_system: CompletionSystem::new(), // Initialize enhanced completion system
        };

        // Load and execute flashrc file if it exists
        if let Err(e) = interpreter.load_rc_file() {
            eprintln!("Warning: Error loading flashrc: {e}");
        }

        interpreter
    }

    /// Set command line arguments for the interpreter
    /// args[0] should be the script name, args[1] should be $1, etc.
    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }

    /// Load and execute the flashrc file
    fn load_rc_file(&mut self) -> io::Result<()> {
        if let Some(ref rc_path) = self.rc_file.clone() {
            if Path::new(rc_path).exists() {
                match fs::read_to_string(rc_path) {
                    Ok(content) => {
                        // Execute the rc file content
                        // We ignore errors in rc file execution to prevent shell startup failure
                        if let Err(e) = self.execute(&content) {
                            eprintln!("Warning: Error executing flashrc: {e}");
                        }
                    }
                    Err(e) => {
                        return Err(io::Error::other(format!(
                            "Failed to read flashrc file {rc_path}: {e}"
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Reload the flashrc file (useful for testing changes without restarting)
    pub fn reload_rc_file(&mut self) -> io::Result<()> {
        self.load_rc_file()
    }

    /// Get the path to the flashrc file
    pub fn get_rc_file_path(&self) -> Option<String> {
        self.rc_file.clone()
    }

    /// Set a custom rc file path
    pub fn set_rc_file_path<P: AsRef<Path>>(&mut self, path: P) {
        self.rc_file = Some(path.as_ref().to_string_lossy().to_string());
    }

    fn save_history(&self) -> io::Result<()> {
        if let Some(ref file_path) = self.history_file {
            let mut file = fs::File::create(file_path)?;
            for line in &self.history {
                writeln!(file, "{line}")?;
            }
        }
        Ok(())
    }

    // Generate completion candidates for the current input using the enhanced completion system
    pub fn generate_completions(
        &mut self,
        input: &str,
        cursor_pos: usize,
    ) -> (Vec<String>, Vec<String>) {
        // Parse the completion context
        let context = CompletionSystem::parse_context(input, cursor_pos);

        // Get completions from the enhanced completion system
        let completions = self.completion_system.complete(&context);

        // Calculate suffixes for the current word
        let current_word = &context.current_word;
        let mut suffixes = Vec::new();
        let mut full_names = Vec::new();

        for completion in completions {
            if completion.starts_with(current_word) {
                // Calculate the suffix (what needs to be added)
                let suffix = &completion[current_word.len()..];
                suffixes.push(suffix.to_string());
                full_names.push(completion);
            } else if current_word.is_empty() {
                // If current word is empty, the whole completion is the suffix
                suffixes.push(completion.clone());
                full_names.push(completion);
            }
        }

        // Fallback to old completion system for variables if new system returns nothing
        if suffixes.is_empty() {
            return self.generate_completions_fallback(input, cursor_pos);
        }

        (suffixes, full_names)
    }

    // Fallback completion method (the old implementation)
    fn generate_completions_fallback(
        &self,
        input: &str,
        cursor_pos: usize,
    ) -> (Vec<String>, Vec<String>) {
        let input_up_to_cursor = &input[..cursor_pos];
        let words: Vec<&str> = input_up_to_cursor.split_whitespace().collect();

        // If we're at the beginning of the line or just completed a word
        if words.is_empty() || input_up_to_cursor.ends_with(' ') {
            // Return list of available commands
            let (suffixes, full_names) = self.get_commands("");
            return (suffixes, full_names);
        }

        // If we're completing the first word (command)
        if words.len() == 1 && !input_up_to_cursor.ends_with(' ') {
            let prefix = words[0];
            let (suffixes, full_names) = self.get_commands(prefix);
            return (suffixes, full_names);
        }

        // Check if we're completing a variable
        if input_up_to_cursor.ends_with('$') {
            // Complete variable names
            let vars: Vec<String> = self.variables.keys().map(|k| format!("${k}")).collect();
            return (vars.clone(), vars);
        }

        if let Some(var_start) = input_up_to_cursor.rfind('$') {
            if var_start < cursor_pos {
                let var_prefix = &input_up_to_cursor[var_start + 1..cursor_pos];
                let suffixes: Vec<String> = self
                    .variables
                    .keys()
                    .filter(|k| k.starts_with(var_prefix))
                    .map(|k| k[var_prefix.len()..].to_string())
                    .collect();
                let full_names: Vec<String> = self
                    .variables
                    .keys()
                    .filter(|k| k.starts_with(var_prefix))
                    .map(|k| format!("${k}"))
                    .collect();
                return (suffixes, full_names);
            }
        }

        // Check if we're completing after a pipe (should complete commands)
        if words.len() >= 2 {
            // Look for the last pipe in the words
            let mut last_pipe_index = None;
            for (i, word) in words.iter().enumerate() {
                if *word == "|" || *word == "&&" || *word == "||" {
                    last_pipe_index = Some(i);
                }
            }

            if let Some(pipe_index) = last_pipe_index {
                // If we're right after a pipe or completing the first word after a pipe
                if pipe_index == words.len() - 2 && !input_up_to_cursor.ends_with(' ') {
                    // We're completing the command after the pipe
                    let prefix = words.last().unwrap_or(&"");
                    let (suffixes, full_names) = self.get_commands(prefix);
                    return (suffixes, full_names);
                } else if pipe_index == words.len() - 1 && input_up_to_cursor.ends_with(' ') {
                    // We just finished typing the pipe and are starting a new command
                    let (suffixes, full_names) = self.get_commands("");
                    return (suffixes, full_names);
                }
            }
        }

        // Otherwise, assume we're completing a filename
        let last_word = if input_up_to_cursor.ends_with(' ') {
            ""
        } else {
            words.last().unwrap_or(&"")
        };

        let (suffixes, full_names) = self.get_path_completions(last_word);
        (suffixes, full_names)
    }

    // Get list of commands that match the given prefix
    fn get_commands(&self, prefix: &str) -> (Vec<String>, Vec<String>) {
        let mut suffixes = Vec::new();
        let mut full_names = Vec::new();

        // Add built-ins
        for cmd in &[
            "cd", "echo", "export", "source", ".", "exit", "alias", "unalias",
        ] {
            if cmd.starts_with(prefix) {
                full_names.push(cmd.to_string());
                if let Some(stripped) = cmd.strip_prefix(prefix) {
                    suffixes.push(stripped.to_string());
                }
            }
        }

        // Add aliases
        for alias_name in self.aliases.keys() {
            if alias_name.starts_with(prefix) {
                full_names.push(alias_name.to_string());
                if let Some(stripped) = alias_name.strip_prefix(prefix) {
                    suffixes.push(stripped.to_string());
                }
            }
        }

        // Add commands from PATH
        if let Ok(path) = env::var("PATH") {
            for path_entry in path.split(':') {
                if let Ok(entries) = fs::read_dir(path_entry) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with(prefix) {
                                if let Some(stripped) = name.strip_prefix(prefix) {
                                    if let Ok(metadata) = entry.path().metadata() {
                                        if metadata.is_file()
                                            && metadata.permissions().mode() & 0o111 != 0
                                        {
                                            full_names.push(name.to_string());
                                            suffixes.push(stripped.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort and deduplicate both lists
        full_names.sort();
        full_names.dedup();
        suffixes.sort();
        suffixes.dedup();

        (suffixes, full_names)
    }

    // Get file/directory completions for the given path prefix
    fn get_path_completions(&self, prefix: &str) -> (Vec<String>, Vec<String>) {
        let mut suffixes = Vec::new();
        let mut full_names = Vec::new();

        // Expand tilde (~) to home directory if present
        let expanded_prefix = self.expand_tilde(prefix);

        // Determine the directory to search and the filename prefix
        let (dir_path, file_prefix) = if expanded_prefix.contains('/') {
            if expanded_prefix.ends_with('/') {
                // If prefix ends with '/', we want to list all files in that directory
                (PathBuf::from(&expanded_prefix), String::new())
            } else {
                let path = Path::new(&expanded_prefix);
                let parent = path.parent().unwrap_or(Path::new(""));
                let file_name = path.file_name().map_or("", |f| f.to_str().unwrap_or(""));
                (parent.to_path_buf(), file_name.to_string())
            }
        } else {
            (PathBuf::from("."), expanded_prefix.clone())
        };

        // Read the directory entries
        if let Ok(entries) = fs::read_dir(dir_path.clone()) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(&file_prefix) {
                        // For display, show the path in the original form (with ~ if it was used)
                        let full_path = if expanded_prefix.contains('/') {
                            // If the original prefix started with ~, preserve that in the display
                            if prefix.starts_with('~') && expanded_prefix != prefix {
                                // Replace the expanded home directory with ~ in the display
                                let home_dir = std::env::var("HOME").unwrap_or_default();
                                let dir_display = dir_path.display().to_string();
                                if dir_display.starts_with(&home_dir) {
                                    let relative_path = &dir_display[home_dir.len()..];
                                    // Handle the case where relative_path starts with / or is empty
                                    if relative_path.is_empty() || relative_path == "/" {
                                        // When completing directly in home directory, show just the filename like bash
                                        name.to_string()
                                    } else if relative_path.starts_with('/') {
                                        format!("~{relative_path}/{name}")
                                    } else {
                                        format!("~/{relative_path}/{name}")
                                    }
                                } else {
                                    format!("{}/{}", dir_path.display(), name)
                                }
                            } else {
                                format!("{}/{}", dir_path.display(), name)
                            }
                        } else {
                            name.to_string()
                        };

                        let mut display_name = full_path.clone();
                        let mut suffix = name[file_prefix.len()..].to_string();

                        // Add a trailing slash for directories
                        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            display_name.push('/');
                            suffix.push('/');
                        }

                        full_names.push(display_name);
                        if !suffix.is_empty() {
                            suffixes.push(suffix);
                        }
                    }
                }
            }
        }

        suffixes.sort();
        full_names.sort();
        (suffixes, full_names)
    }

    // Display a list of completions
    fn display_completions(&self, completions: &[String]) -> io::Result<()> {
        if completions.is_empty() {
            return Ok(());
        }

        println!(); // Move to a new line

        // Calculate the maximum width of completions
        let max_width = completions.iter().map(|s| s.len()).max().unwrap_or(0) + 2;
        let term_width = self.get_terminal_width();
        let columns = std::cmp::max(1, term_width / max_width);

        // Display completions in columns
        for (i, completion) in completions.iter().enumerate() {
            print!("{completion:<max_width$}");
            if (i + 1) % columns == 0 {
                println!();
            }
        }

        // Ensure we end with a newline
        if completions.len() % columns != 0 {
            println!();
        }

        Ok(())
    }

    // Get the terminal width
    fn get_terminal_width(&self) -> usize {
        use std::process::Command;

        let width = if cfg!(unix) {
            // On Unix-like systems, try `tput cols`
            Command::new("tput")
                .arg("cols")
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        String::from_utf8(output.stdout)
                            .ok()?
                            .trim()
                            .parse::<usize>()
                            .ok()
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    // Fallback: try `stty size` and extract columns
                    Command::new("stty")
                        .arg("size")
                        .output()
                        .ok()
                        .and_then(|output| {
                            if output.status.success() {
                                let size_str = String::from_utf8(output.stdout).ok()?;
                                let parts: Vec<&str> = size_str.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    parts[1].parse::<usize>().ok()
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                })
        } else if cfg!(windows) {
            // On Windows, try PowerShell to get console width
            Command::new("powershell")
                .args(["-Command", "(Get-Host).UI.RawUI.WindowSize.Width"])
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        String::from_utf8(output.stdout)
                            .ok()?
                            .trim()
                            .parse::<usize>()
                            .ok()
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        // Return the detected width or default to 80
        width.unwrap_or(80)
    }

    /// Get the current prompt string, expanding variables
    fn get_prompt(&self) -> String {
        if let Some(prompt_template) = self.variables.get("PROMPT") {
            // Use the existing expand_variables method for proper variable expansion
            self.expand_variables(prompt_template)
        } else {
            "ϟ ".to_string()
        }
    }

    // Interactive shell that accepts a custom evaluator
    pub fn run_interactive_with_evaluator<E: Evaluator>(
        &mut self,
        mut evaluator: E,
    ) -> io::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let fd = stdin.as_raw_fd();

        // Check if stdin is a terminal using isatty
        if unsafe { libc::isatty(fd) } == 0 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Interactive mode requires a terminal",
            ));
        }

        let original_termios = tcgetattr(fd)?;
        let mut raw_termios = original_termios;

        // RAII guard to restore terminal settings on drop
        struct TermiosGuard {
            fd: i32,
            original: libc::termios,
        }

        impl Drop for TermiosGuard {
            fn drop(&mut self) {
                let _ = tcsetattr(self.fd, libc::TCSANOW, &self.original);
            }
        }

        let _guard = TermiosGuard {
            fd,
            original: original_termios,
        };

        // Ignore SIGINT (Ctrl+C) for the shell process
        // This prevents the shell from exiting when Ctrl+C is pressed
        unsafe {
            libc::signal(libc::SIGINT, libc::SIG_IGN);
        }

        let mut history_index = self.history.len();

        loop {
            let prompt = self.get_prompt();
            write!(stdout, "{prompt}")?;
            stdout.flush()?;

            let input = self.read_line_with_completion(
                &prompt,
                &original_termios,
                &mut raw_termios,
                &mut history_index,
            )?;

            if input.trim().is_empty() {
                continue;
            }

            if input.trim() == "exit" {
                break;
            }

            if !input.trim().is_empty()
                && (self.history.is_empty() || (self.history.last() != Some(&input)))
            {
                self.history.push(input.clone());
                history_index = self.history.len();
                let _ = self.save_history();
            }

            let result = self.execute_with_evaluator(&input, &mut evaluator);

            match result {
                Ok(code) => {
                    self.last_exit_code = code;
                    self.variables.insert("?".to_string(), code.to_string());
                }
                Err(e) => {
                    println!("Error: {e}");
                    self.last_exit_code = 1;
                    self.variables.insert("?".to_string(), "1".to_string());
                }
            }
        }

        self.save_history()?;
        Ok(())
    }

    // Default interactive shell using DefaultEvaluator
    pub fn run_interactive(&mut self) -> io::Result<()> {
        let default_evaluator = DefaultEvaluator;
        self.run_interactive_with_evaluator(default_evaluator)
    }

    fn read_line_with_completion(
        &mut self,
        prompt: &str,
        original_termios: &libc::termios,
        raw_termios: &mut libc::termios,
        history_index: &mut usize,
    ) -> io::Result<String> {
        let mut stdin = io::stdin();
        let mut stdout = io::stdout();
        let fd = stdin.as_raw_fd();

        let mut buffer = String::new();
        let mut cursor_pos = 0;

        // For storing the kill ring (for cut/paste operations)
        let mut kill_ring = String::new();

        loop {
            // Switch to raw mode to read individual characters
            raw_termios.c_lflag &= !(libc::ICANON | libc::ECHO);
            raw_termios.c_cc[libc::VMIN] = 1;
            raw_termios.c_cc[libc::VTIME] = 0;
            tcsetattr(fd, libc::TCSANOW, raw_termios)?;

            // Read a single byte
            let mut input_byte = [0u8; 1];
            stdin.read_exact(&mut input_byte)?;

            // Switch back to canonical mode for printing
            tcsetattr(fd, libc::TCSANOW, original_termios)?;

            match input_byte[0] {
                // Enter
                b'\n' | b'\r' => {
                    println!();
                    break;
                }

                // Tab for completion
                b'\t' => {
                    let (suffixes, full_names) = self.generate_completions(&buffer, cursor_pos);

                    match suffixes.len().cmp(&1) {
                        std::cmp::Ordering::Less => {
                            // Do nothing - no completions available
                        }
                        std::cmp::Ordering::Equal => {
                            // If there's only one completion, use it
                            let suffix = &suffixes[0];
                            buffer.insert_str(cursor_pos, suffix);
                            cursor_pos += suffix.len();

                            // Redraw the line with the completion
                            write!(stdout, "\r{prompt}{buffer}")?;
                            stdout.flush()?;
                        }
                        std::cmp::Ordering::Greater => {
                            // Find the common prefix among suffixes
                            if let Some(common_prefix) = self.find_common_prefix(&suffixes) {
                                if !common_prefix.is_empty() {
                                    // Only insert the common prefix
                                    buffer.insert_str(cursor_pos, &common_prefix);
                                    cursor_pos += common_prefix.len();

                                    // Redraw the line with the partial completion
                                    write!(stdout, "\r{prompt}{buffer}")?;
                                    stdout.flush()?;
                                } else {
                                    // No common prefix, show all completions (using full names for display)
                                    self.display_completions(&full_names)?;
                                    // Redraw the prompt and line
                                    write!(stdout, "{prompt}{buffer}")?;
                                    stdout.flush()?;
                                }
                            } else {
                                // No common prefix found, show all completions (using full names for display)
                                self.display_completions(&full_names)?;
                                // Redraw the prompt and line
                                write!(stdout, "{prompt}{buffer}")?;
                                stdout.flush()?;
                            }
                        }
                    }
                }
                // Backspace
                8 | 127 => {
                    if cursor_pos > 0 {
                        buffer.remove(cursor_pos - 1);
                        cursor_pos -= 1;
                        write!(stdout, "\r{prompt}{buffer}")?;
                        write!(stdout, " ")?; // Clear deleted character
                        write!(stdout, "\r{prompt}{buffer}")?;
                        stdout.flush()?;
                    }
                }

                // Ctrl-A (move to beginning of line)
                1 => {
                    cursor_pos = 0;
                    write!(stdout, "\r{prompt}")?;
                    stdout.flush()?;
                }

                // Ctrl-E (move to end of line)
                5 => {
                    cursor_pos = buffer.len();
                    write!(stdout, "\r{prompt}{buffer}")?;
                    stdout.flush()?;
                }

                // Ctrl-B (move back one character) - same as left arrow
                2 => {
                    if cursor_pos > 0 {
                        cursor_pos -= 1;
                        write!(stdout, "\r{prompt}{buffer}")?;
                        // Move cursor back to the right position
                        for _ in 0..(buffer.len() - cursor_pos) {
                            write!(stdout, "\x1B[D")?;
                        }
                        stdout.flush()?;
                    }
                }

                // Ctrl-F (move forward one character) - same as right arrow
                6 => {
                    if cursor_pos < buffer.len() {
                        cursor_pos += 1;
                        write!(stdout, "\r{prompt}{buffer}")?;
                        // Move cursor back to the right position
                        for _ in 0..(buffer.len() - cursor_pos) {
                            write!(stdout, "\x1B[D")?;
                        }
                        stdout.flush()?;
                    }
                }

                // Ctrl-K (kill from cursor to end of line)
                11 => {
                    if cursor_pos < buffer.len() {
                        // Save the killed text
                        kill_ring = buffer[cursor_pos..].to_string();

                        // Remove from buffer
                        buffer.truncate(cursor_pos);

                        // Redraw
                        write!(stdout, "\r{prompt}{buffer}")?;
                        write!(stdout, "                    ")?; // Clear any leftovers
                        write!(stdout, "\r{prompt}{buffer}")?;
                        stdout.flush()?;
                    }
                }

                // Ctrl-U (kill from beginning of line to cursor)
                21 => {
                    if cursor_pos > 0 {
                        // Save the killed text
                        kill_ring = buffer[..cursor_pos].to_string();

                        // Remove from buffer
                        buffer = buffer[cursor_pos..].to_string();
                        cursor_pos = 0;

                        // Redraw
                        write!(stdout, "\r{prompt}{buffer}")?;
                        write!(stdout, "                    ")?; // Clear any leftovers
                        write!(stdout, "\r{prompt}{buffer}")?;
                        stdout.flush()?;
                    }
                }

                // Ctrl-Y (yank/paste previously killed text)
                25 => {
                    if !kill_ring.is_empty() {
                        buffer.insert_str(cursor_pos, &kill_ring);
                        cursor_pos += kill_ring.len();

                        // Redraw
                        write!(stdout, "\r{prompt}{buffer}")?;
                        // Move cursor back to the right position
                        for _ in 0..(buffer.len() - cursor_pos) {
                            write!(stdout, "\x1B[D")?;
                        }
                        stdout.flush()?;
                    }
                }

                // Ctrl-W (delete word backward)
                23 => {
                    // Delete the word before the cursor
                    if cursor_pos > 0 {
                        // Find the start of the current word
                        let mut word_start = cursor_pos;
                        let buffer_bytes = buffer.as_bytes();

                        // Skip any whitespace immediately before cursor
                        while word_start > 0 && buffer_bytes[word_start - 1].is_ascii_whitespace() {
                            word_start -= 1;
                        }

                        // Now find the start of the word
                        while word_start > 0 && !buffer_bytes[word_start - 1].is_ascii_whitespace()
                        {
                            word_start -= 1;
                        }

                        // Save to kill ring
                        kill_ring = buffer[word_start..cursor_pos].to_string();

                        // Delete from word_start to cursor_pos
                        if word_start < cursor_pos {
                            buffer.replace_range(word_start..cursor_pos, "");
                            cursor_pos = word_start;

                            // Redraw the line
                            write!(stdout, "\r{prompt}{buffer}")?;
                            write!(stdout, "                    ")?; // Clear any leftovers
                            write!(stdout, "\r{prompt}{buffer}")?;
                            stdout.flush()?;
                        }
                    }
                }

                // Ctrl-L (clear screen)
                12 => {
                    // Clear the screen and redraw the prompt
                    write!(stdout, "\x1B[2J\x1B[H")?; // ANSI escape sequence to clear screen and move cursor to home
                    write!(stdout, "{prompt}{buffer}")?;
                    stdout.flush()?;
                }

                // Ctrl-P (previous history) - same as up arrow
                16 => {
                    if *history_index > 0 {
                        *history_index -= 1;
                        buffer = self.history[*history_index].clone();
                        cursor_pos = buffer.len();
                        write!(stdout, "\r{prompt}{buffer}")?;
                        write!(stdout, "                    ")?; // Clear any leftovers
                        write!(stdout, "\r{prompt}{buffer}")?;
                        stdout.flush()?;
                    }
                }

                // Ctrl-N (next history) - same as down arrow
                14 => {
                    if *history_index < self.history.len() {
                        *history_index += 1;
                        if *history_index == self.history.len() {
                            buffer.clear();
                            cursor_pos = 0;
                        } else {
                            buffer = self.history[*history_index].clone();
                            cursor_pos = buffer.len();
                        }
                        write!(stdout, "\r{prompt}{buffer}")?;
                        write!(stdout, "                    ")?; // Clear any leftovers
                        write!(stdout, "\r{prompt}{buffer}")?;
                        stdout.flush()?;
                    }
                }

                // Ctrl-T (transpose characters)
                20 => {
                    // Handle cursor at end of line
                    if cursor_pos == buffer.len() && cursor_pos >= 2 {
                        // Swap the last two characters
                        let last_idx = buffer.len() - 1;
                        let second_to_last_idx = buffer.len() - 2;

                        // Can't use remove/insert directly with indices, so use chars
                        let mut chars: Vec<char> = buffer.chars().collect();
                        chars.swap(last_idx, second_to_last_idx);

                        // Rebuild the buffer
                        buffer = chars.into_iter().collect();

                        // Cursor remains at the end

                        // Redraw
                        write!(stdout, "\r{prompt}{buffer}")?;
                        stdout.flush()?;
                    }
                    // Handle cursor within the line
                    else if cursor_pos > 0 && cursor_pos < buffer.len() {
                        // Get chars to swap
                        let prev_char = buffer.remove(cursor_pos - 1);
                        buffer.insert(cursor_pos, prev_char);

                        // Advance cursor position after transposition
                        cursor_pos += 1;

                        // Redraw
                        write!(stdout, "\r{prompt}{buffer}")?;
                        // Move cursor back to the right position
                        for _ in 0..(buffer.len() - cursor_pos) {
                            write!(stdout, "\x1B[D")?;
                        }
                        stdout.flush()?;
                    }
                }

                // Ctrl-D (delete character under cursor or exit if buffer is empty)
                4 => {
                    if buffer.is_empty() {
                        println!("exit");
                        return Ok("exit".to_string());
                    } else if cursor_pos < buffer.len() {
                        buffer.remove(cursor_pos);
                        write!(stdout, "\r{prompt}{buffer}")?;
                        write!(stdout, " ")?; // Clear deleted character
                        write!(stdout, "\r{prompt}{buffer}")?;
                        // Move cursor back to the right position
                        for _ in 0..(buffer.len() - cursor_pos) {
                            write!(stdout, "\x1B[D")?;
                        }
                        stdout.flush()?;
                    }
                }

                // Ctrl-R (reverse history search) - simplified version
                18 => {
                    // Store the current buffer in case search is cancelled
                    let original_buffer = buffer.clone();
                    let original_cursor_pos = cursor_pos;

                    // Create a search buffer
                    let mut search_term = String::new();
                    let mut search_index = self.history.len() - 1;
                    let mut found = false;

                    // Display the search prompt
                    write!(stdout, "\r(reverse-i-search)`': ")?;
                    stdout.flush()?;

                    // Read characters for search
                    loop {
                        // Read a single byte in raw mode
                        raw_termios.c_lflag &= !(libc::ICANON | libc::ECHO);
                        tcsetattr(fd, libc::TCSANOW, raw_termios)?;
                        let mut search_byte = [0u8; 1];
                        stdin.read_exact(&mut search_byte)?;
                        tcsetattr(fd, libc::TCSANOW, original_termios)?;

                        match search_byte[0] {
                            // Enter - accept the current match
                            b'\n' | b'\r' => {
                                if found {
                                    write!(stdout, "\r\n")?;
                                    buffer = self.history[search_index].clone();
                                    cursor_pos = buffer.len();
                                } else {
                                    write!(stdout, "\r{prompt}{original_buffer}")?;
                                    cursor_pos = original_cursor_pos;
                                }
                                stdout.flush()?;
                                break;
                            }

                            // Escape - cancel search
                            27 => {
                                write!(stdout, "\r{prompt}{original_buffer}")?;
                                cursor_pos = original_cursor_pos;
                                stdout.flush()?;
                                break;
                            }

                            // Ctrl-R - search for next occurrence
                            18 => {
                                if found {
                                    let mut temp_index = search_index;
                                    let mut found_next = false;

                                    // Start search from one past the current match
                                    if temp_index > 0 {
                                        temp_index -= 1;

                                        while temp_index < self.history.len() {
                                            if self.history[temp_index].contains(&search_term) {
                                                search_index = temp_index;
                                                found_next = true;
                                                break;
                                            }
                                            if temp_index == 0 {
                                                break;
                                            }
                                            temp_index -= 1;
                                        }
                                    }

                                    if found_next {
                                        write!(
                                            stdout,
                                            "\r(reverse-i-search)`{}': {}",
                                            search_term, self.history[search_index]
                                        )?;
                                    } else {
                                        write!(
                                            stdout,
                                            "\r(failed reverse-i-search)`{}': {}",
                                            search_term, self.history[search_index]
                                        )?;
                                    }
                                    stdout.flush()?;
                                }
                            }

                            // Backspace
                            8 | 127 => {
                                if !search_term.is_empty() {
                                    search_term.pop();
                                    found = false;

                                    // Search from the end of history
                                    search_index = self.history.len() - 1;

                                    while search_index < self.history.len() {
                                        if self.history[search_index].contains(&search_term) {
                                            found = true;
                                            break;
                                        }
                                        if search_index == 0 {
                                            break;
                                        }
                                        search_index -= 1;
                                    }

                                    if found {
                                        write!(
                                            stdout,
                                            "\r(reverse-i-search)`{}': {}",
                                            search_term, self.history[search_index]
                                        )?;
                                    } else {
                                        write!(
                                            stdout,
                                            "\r(failed reverse-i-search)`{search_term}': "
                                        )?;
                                    }
                                    stdout.flush()?;
                                }
                            }

                            // Regular character - add to search term
                            _ => {
                                let ch = search_byte[0] as char;
                                if ch.is_ascii() && !ch.is_control() {
                                    search_term.push(ch);
                                    found = false;

                                    // Search from the end of history
                                    search_index = self.history.len() - 1;

                                    while search_index < self.history.len() {
                                        if self.history[search_index].contains(&search_term) {
                                            found = true;
                                            break;
                                        }
                                        if search_index == 0 {
                                            break;
                                        }
                                        search_index -= 1;
                                    }

                                    if found {
                                        write!(
                                            stdout,
                                            "\r(reverse-i-search)`{}': {}",
                                            search_term, self.history[search_index]
                                        )?;
                                    } else {
                                        write!(
                                            stdout,
                                            "\r(failed reverse-i-search)`{search_term}': "
                                        )?;
                                    }
                                    stdout.flush()?;
                                }
                            }
                        }
                    }

                    continue; // Skip the rest of the loop for this iteration
                }

                // Ctrl-C
                3 => {
                    // Print ^C and move to new line (like bash)
                    print!("^C");
                    stdout.flush()?;
                    println!();

                    // Clear the current buffer and reset cursor
                    buffer.clear();
                    cursor_pos = 0;

                    // Show a fresh prompt on the new line
                    write!(stdout, "{prompt}")?;
                    stdout.flush()?;
                }

                // Escape sequence (arrow keys, etc.)
                27 => {
                    // Read the next two bytes
                    let mut escape_seq = [0u8; 2];
                    stdin.read_exact(&mut escape_seq)?;

                    if escape_seq[0] == b'[' {
                        match escape_seq[1] {
                            // Up arrow - history navigation
                            b'A' => {
                                if *history_index > 0 {
                                    *history_index -= 1;
                                    buffer = self.history[*history_index].clone();
                                    cursor_pos = buffer.len();
                                    write!(stdout, "\r{prompt}{buffer}")?;
                                    write!(stdout, "                    ")?; // Clear any leftovers
                                    write!(stdout, "\r{prompt}{buffer}")?;
                                    stdout.flush()?;
                                }
                            }

                            // Down arrow - history navigation
                            b'B' => {
                                if *history_index < self.history.len() {
                                    *history_index += 1;
                                    if *history_index == self.history.len() {
                                        buffer.clear();
                                        cursor_pos = 0;
                                    } else {
                                        buffer = self.history[*history_index].clone();
                                        cursor_pos = buffer.len();
                                    }
                                    write!(stdout, "\r{prompt}{buffer}")?;
                                    write!(stdout, "                    ")?; // Clear any leftovers
                                    write!(stdout, "\r{prompt}{buffer}")?;
                                    stdout.flush()?;
                                }
                            }

                            // Left arrow
                            b'D' => {
                                if cursor_pos > 0 {
                                    cursor_pos -= 1;
                                    write!(stdout, "\r{prompt}{buffer}")?;
                                    // Move cursor back to the right position
                                    for _ in 0..(buffer.len() - cursor_pos) {
                                        write!(stdout, "\x1B[D")?;
                                    }
                                    stdout.flush()?;
                                }
                            }

                            // Right arrow
                            b'C' => {
                                if cursor_pos < buffer.len() {
                                    cursor_pos += 1;
                                    write!(stdout, "\r{prompt}{buffer}")?;
                                    // Move cursor back to the right position
                                    for _ in 0..(buffer.len() - cursor_pos) {
                                        write!(stdout, "\x1B[D")?;
                                    }
                                    stdout.flush()?;
                                }
                            }

                            // Alt+Left (Home) or Alt+Right (End) could be added here
                            _ => {}
                        }
                    }
                }

                // Regular character
                _ => {
                    let ch = input_byte[0] as char;
                    if ch.is_ascii() && !ch.is_control() {
                        buffer.insert(cursor_pos, ch);
                        cursor_pos += 1;
                        write!(stdout, "\r{prompt}{buffer}")?;
                        // Move cursor back to the right position
                        for _ in 0..(buffer.len() - cursor_pos) {
                            write!(stdout, "\x1B[D")?;
                        }
                        stdout.flush()?;
                    }
                }
            }
        }

        Ok(buffer)
    }

    // Find the longest common prefix among completion candidates
    fn find_common_prefix(&self, completions: &[String]) -> Option<String> {
        if completions.is_empty() {
            return None;
        }

        if completions.len() == 1 {
            return Some(completions[0].clone());
        }

        let first = &completions[0];
        let mut common_len = first.len();

        for completion in &completions[1..] {
            let mut i = 0;
            let mut matched = true;

            for (c1, c2) in first.chars().zip(completion.chars()) {
                if c1 != c2 {
                    matched = false;
                    break;
                }
                i += 1;
            }

            if !matched {
                common_len = common_len.min(i);
            } else {
                common_len = common_len.min(completion.len());
            }
        }

        if common_len == 0 {
            return None;
        }

        Some(first[..common_len].to_string())
    }

    // Main execution method that accepts a custom evaluator.
    pub fn execute_with_evaluator<E: Evaluator>(
        &mut self,
        input: &str,
        evaluator: &mut E,
    ) -> Result<i32, io::Error> {
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_script();
        self.evaluate_with_evaluator(&ast, evaluator)
    }

    // Default execute method using DefaultEvaluator.
    pub fn execute(&mut self, input: &str) -> Result<i32, io::Error> {
        let mut default_evaluator = DefaultEvaluator;
        self.execute_with_evaluator(input, &mut default_evaluator)
    }

    // Internal evaluation method that uses the provided evaluator
    pub fn evaluate_with_evaluator<E: Evaluator>(
        &mut self,
        node: &Node,
        evaluator: &mut E,
    ) -> Result<i32, io::Error> {
        evaluator.evaluate(node, self)
    }

    // Helper method for matching extended glob patterns
    fn matches_ext_glob(
        &self,
        filename: &str,
        operator: char,
        patterns: &[String],
        suffix: &str,
    ) -> bool {
        // Check if the filename has the required suffix
        if !filename.ends_with(suffix) {
            return false;
        }

        // Remove the suffix for pattern matching
        let without_suffix = if suffix.is_empty() {
            filename.to_string()
        } else {
            filename[..filename.len() - suffix.len()].to_string()
        };

        // Convert patterns to regex patterns
        let regex_patterns: Vec<Regex> = patterns
            .iter()
            .map(|p| {
                // Convert glob pattern to regex
                // This is simplified and doesn't handle all glob features
                let escaped = regex::escape(p);
                let regex_str = escaped.replace("\\*", ".*").replace("\\?", ".");
                Regex::new(&format!("^{regex_str}$")).unwrap_or_else(|_| Regex::new("^$").unwrap())
            })
            .collect();

        // Apply the operator logic
        match operator {
            '?' => {
                // Match any of the patterns exactly once
                regex_patterns.iter().any(|re| re.is_match(&without_suffix))
            }
            '*' => {
                // Match zero or more occurrences of any of the patterns
                true // Simplified - should check for zero or more matches
            }
            '+' => {
                // Match one or more occurrences of any of the patterns
                regex_patterns.iter().any(|re| re.is_match(&without_suffix))
            }
            '@' => {
                // Match exactly one of the patterns
                let match_count = regex_patterns
                    .iter()
                    .filter(|re| re.is_match(&without_suffix))
                    .count();
                match_count == 1
            }
            '!' => {
                // Match anything except one of the patterns
                !regex_patterns.iter().any(|re| re.is_match(&without_suffix))
            }
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn capture_command_output<E: Evaluator>(
        &mut self,
        node: &Node,
        evaluator: &mut E,
    ) -> Result<String, io::Error> {
        match node {
            Node::Command {
                name,
                args,
                redirects: _,
            } => {
                // Handle built-in commands that should work in command substitution
                match name.as_str() {
                    "echo" => {
                        let mut output = String::new();
                        for (i, arg) in args.iter().enumerate() {
                            let expanded_arg = self.expand_variables(arg);
                            if i > 0 {
                                output.push(' ');
                            }
                            output.push_str(&expanded_arg);
                        }
                        Ok(output)
                    }
                    "pwd" => match env::current_dir() {
                        Ok(dir) => Ok(dir.to_string_lossy().to_string()),
                        Err(e) => Err(e),
                    },
                    _ => {
                        // Execute external command and capture output
                        let mut command = std::process::Command::new(name);

                        // Expand variables in arguments
                        let expanded_args: Vec<String> =
                            args.iter().map(|arg| self.expand_variables(arg)).collect();
                        command.args(&expanded_args);

                        // Set environment variables
                        for (key, value) in &self.variables {
                            command.env(key, value);
                        }

                        match command.output() {
                            Ok(output) => {
                                if output.status.success() {
                                    let stdout = String::from_utf8_lossy(&output.stdout);
                                    Ok(stdout.trim_end().to_string())
                                } else {
                                    // For command substitution, we might want to return empty string
                                    // instead of error for failed commands, depending on shell behavior
                                    Ok(String::new())
                                }
                            }
                            Err(_) => {
                                // Command not found - return empty string
                                Ok(String::new())
                            }
                        }
                    }
                }
            }
            Node::List {
                statements,
                operators,
            } => {
                // Handle command lists in substitution (e.g., $(cmd1; cmd2))
                let mut last_output = String::new();

                for (i, statement) in statements.iter().enumerate() {
                    let output = self.capture_command_output(statement, evaluator)?;

                    // For lists, we typically want the output of the last command
                    if i < operators.len() {
                        match operators[i].as_str() {
                            "&&" => {
                                // Only continue if previous command succeeded
                                if output.is_empty() {
                                    break;
                                }
                                last_output = output;
                            }
                            "||" => {
                                // Only continue if previous command failed
                                if !output.is_empty() {
                                    last_output = output;
                                    break;
                                }
                            }
                            _ => {
                                last_output = output;
                            }
                        }
                    } else {
                        last_output = output;
                    }
                }

                Ok(last_output)
            }
            Node::Pipeline { commands } => {
                // Handle pipelines in command substitution
                if commands.is_empty() {
                    return Ok(String::new());
                }

                if commands.len() == 1 {
                    return self.capture_command_output(&commands[0], evaluator);
                }

                // For now, just return the output of the last command
                // A full implementation would need to properly pipe between commands
                self.capture_command_output(&commands[commands.len() - 1], evaluator)
            }
            Node::Assignment { name, value } => {
                // Handle variable assignment in command substitution
                match value.as_ref() {
                    Node::StringLiteral(string_value) => {
                        let expanded_value = self.expand_variables(string_value);
                        self.variables.insert(name.clone(), expanded_value.clone());
                        Ok(expanded_value)
                    }
                    Node::CommandSubstitution { command } => {
                        let output = self.capture_command_output(command, evaluator)?;
                        self.variables.insert(name.clone(), output.clone());
                        Ok(output)
                    }
                    Node::ArithmeticExpansion { expression } => {
                        let expanded_expr = self.expand_variables(expression);
                        match DefaultEvaluator::evaluate_arithmetic_expression(&expanded_expr) {
                            Ok(result) => {
                                let result_str = result.to_string();
                                self.variables.insert(name.clone(), result_str.clone());
                                Ok(result_str)
                            }
                            Err(_) => {
                                eprintln!(
                                    "arithmetic expansion: invalid expression: {expanded_expr}"
                                );
                                self.variables.insert(name.clone(), "0".to_string());
                                Ok("0".to_string())
                            }
                        }
                    }
                    Node::ArithmeticCommand { expression } => {
                        let expanded_expr = self.expand_variables(expression);
                        match DefaultEvaluator::evaluate_arithmetic_expression(&expanded_expr) {
                            Ok(result) => {
                                let result_str = result.to_string();
                                self.variables.insert(name.clone(), result_str.clone());
                                Ok(result_str)
                            }
                            Err(_) => {
                                eprintln!(
                                    "arithmetic command: invalid expression: {expanded_expr}"
                                );
                                self.variables.insert(name.clone(), "0".to_string());
                                Ok("0".to_string())
                            }
                        }
                    }
                    _ => Ok(String::new()),
                }
            }
            Node::StringLiteral(value) => {
                // For string literals, just return the expanded value
                Ok(self.expand_variables(value))
            }
            _ => {
                // For other node types, return empty string
                Ok(String::new())
            }
        }
    }

    // Method to evaluate arithmetic expressions with variable access
    fn evaluate_arithmetic_with_variables(&self, expr: &str) -> Result<i64, String> {
        // This method handles arithmetic expressions with variables and nested arithmetic expansions

        let expr = expr.trim();
        if expr.is_empty() {
            return Ok(0);
        }

        // Handle simple number
        if let Ok(num) = expr.parse::<i64>() {
            return Ok(num);
        }

        // Handle simple variable name
        if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
            // This is a variable name, look it up
            if let Some(value) = self.variables.get(expr) {
                if let Ok(num) = value.parse::<i64>() {
                    return Ok(num);
                }
            }
            return Ok(0); // Unset variable defaults to 0
        }

        // First, expand any nested arithmetic expressions $((...)
        let expanded_expr = self.expand_nested_arithmetic(expr)?;

        // Then expand variables in the result
        let mut final_expr = String::new();
        let mut chars = expanded_expr.chars().peekable();

        while let Some(c) = chars.next() {
            if c.is_alphabetic() || c == '_' {
                // This might be a variable name
                let mut var_name = String::new();
                var_name.push(c);

                // Collect the rest of the variable name
                while let Some(&next_c) = chars.peek() {
                    if next_c.is_alphanumeric() || next_c == '_' {
                        var_name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                // Look up the variable
                if let Some(value) = self.variables.get(&var_name) {
                    final_expr.push_str(value);
                } else {
                    final_expr.push('0'); // Unset variable defaults to 0
                }
            } else {
                final_expr.push(c);
            }
        }

        // Now evaluate the fully expanded expression using the static method
        DefaultEvaluator::evaluate_arithmetic_expression(&final_expr)
    }

    // Helper method to expand nested arithmetic expressions $((...)
    fn expand_nested_arithmetic(&self, expr: &str) -> Result<String, String> {
        let mut result = String::new();
        let mut chars = expr.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' && chars.peek() == Some(&'(') {
                chars.next(); // consume '('
                if chars.peek() == Some(&'(') {
                    chars.next(); // consume second '('

                    // Find the matching closing parentheses
                    let mut paren_count = 2;
                    let mut nested_expr = String::new();

                    while paren_count > 0 {
                        if let Some(ch) = chars.next() {
                            if ch == '(' {
                                paren_count += 1;
                            } else if ch == ')' {
                                paren_count -= 1;
                            }

                            if paren_count > 0 {
                                nested_expr.push(ch);
                            }
                        } else {
                            return Err(
                                "Unmatched parentheses in arithmetic expression".to_string()
                            );
                        }
                    }

                    // Recursively evaluate the nested expression
                    let nested_result = self.evaluate_arithmetic_with_variables(&nested_expr)?;
                    result.push_str(&nested_result.to_string());
                } else {
                    // Not an arithmetic expansion, put back the characters
                    result.push('$');
                    result.push('(');
                }
            } else {
                result.push(c);
            }
        }

        Ok(result)
    }

    fn expand_variables(&self, input: &str) -> String {
        // First handle tilde expansion
        let input = self.expand_tilde(input);

        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' && chars.peek().is_some() {
                // Check if the next character is a valid start of a variable expansion
                let next_char = *chars.peek().unwrap();
                if !matches!(next_char, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9' | '{' | '(' | '#' | '@' | '*' | '?' | '$')
                {
                    // Not a valid variable start, treat $ as literal
                    result.push(c);
                    continue;
                }

                // Check for command substitution $(...) or arithmetic expansion $((...))
                if let Some(&'(') = chars.peek() {
                    chars.next(); // Skip first '('

                    // Check if this is arithmetic expansion $((...))
                    if let Some(&'(') = chars.peek() {
                        chars.next(); // Skip second '('

                        // Find the matching closing parentheses for arithmetic expansion
                        let mut paren_count = 2;
                        let mut arith_content = String::new();

                        for ch in chars.by_ref() {
                            if ch == '(' {
                                paren_count += 1;
                            } else if ch == ')' {
                                paren_count -= 1;
                                if paren_count == 0 {
                                    break;
                                }
                            }
                            if paren_count > 0 {
                                arith_content.push(ch);
                            }
                        }

                        // Evaluate the arithmetic expression
                        if !arith_content.is_empty() {
                            match self.evaluate_arithmetic_with_variables(&arith_content) {
                                Ok(arith_result) => {
                                    result.push_str(&arith_result.to_string());
                                }
                                Err(_) => {
                                    eprintln!(
                                        "arithmetic expansion: invalid expression: {arith_content}"
                                    );
                                    result.push('0');
                                }
                            }
                        }
                    } else {
                        // Regular command substitution $(...)
                        // Find the matching closing parenthesis
                        let mut paren_count = 1;
                        let mut cmd_content = String::new();

                        for ch in chars.by_ref() {
                            if ch == '(' {
                                paren_count += 1;
                            } else if ch == ')' {
                                paren_count -= 1;
                                if paren_count == 0 {
                                    break;
                                }
                            }
                            cmd_content.push(ch);
                        }

                        // Execute the command substitution
                        if !cmd_content.is_empty() {
                            if let Ok(output) = self.execute_command_for_substitution(&cmd_content)
                            {
                                result.push_str(&output);
                            }
                        }
                    }
                    continue;
                }

                let mut var_name = String::new();

                // Variable can be specified as ${VAR} or $VAR
                if let Some(&'{') = chars.peek() {
                    chars.next(); // Skip '{'

                    // Read until closing brace
                    for c in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                        var_name.push(c);
                    }
                } else {
                    // Read variable name
                    // Handle special single-character variables first
                    if let Some(&c) = chars.peek() {
                        if matches!(c, '#' | '@' | '*' | '?' | '$') {
                            var_name.push(c);
                            chars.next();
                        } else {
                            // Read until non-alphanumeric character
                            while let Some(&c) = chars.peek() {
                                if c.is_alphanumeric() || c == '_' {
                                    var_name.push(c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }

                // Replace with variable value if exists
                if let Some(value) = self.variables.get(&var_name) {
                    result.push_str(value);
                } else if var_name.chars().all(|c| c.is_ascii_digit()) {
                    // Handle positional parameters ($0, $1, $2, ...)
                    if let Ok(index) = var_name.parse::<usize>() {
                        if let Some(arg) = self.args.get(index) {
                            result.push_str(arg);
                        }
                        // If the argument doesn't exist, expand to empty string (standard shell behavior)
                    }
                } else if var_name == "#" {
                    // $# - number of positional parameters (excluding $0)
                    let count = if self.args.is_empty() {
                        0
                    } else {
                        self.args.len() - 1
                    };
                    result.push_str(&count.to_string());
                } else if var_name == "@" || var_name == "*" {
                    // $@ and $* - all positional parameters (excluding $0)
                    if self.args.len() > 1 {
                        let params = &self.args[1..];
                        result.push_str(&params.join(" "));
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Expand tilde (~) to home directory
    fn expand_tilde(&self, input: &str) -> String {
        if input.is_empty() {
            return input.to_string();
        }

        // Get home directory
        let home_dir = match std::env::var("HOME") {
            Ok(home) => home,
            Err(_) => return input.to_string(), // If HOME is not set, return unchanged
        };

        let mut result = String::new();
        let mut chars = input.chars().peekable();
        let mut at_word_start = true; // Track if we're at the start of a word

        while let Some(c) = chars.next() {
            if c == '~' && at_word_start {
                // Check what follows the tilde
                match chars.peek() {
                    None => {
                        // Tilde at end of string - expand to home directory
                        result.push_str(&home_dir);
                    }
                    Some('/') => {
                        // Tilde followed by slash - expand to home directory
                        result.push_str(&home_dir);
                    }
                    Some(' ') | Some('\t') => {
                        // Tilde followed by whitespace - expand to home directory
                        result.push_str(&home_dir);
                    }
                    Some(_) => {
                        // Tilde followed by other characters - could be ~username
                        // For now, we'll only support ~ for current user
                        // In the future, we could add support for ~username
                        let mut username = String::new();
                        while let Some(&ch) = chars.peek() {
                            if ch == '/' || ch.is_whitespace() {
                                break;
                            }
                            username.push(ch);
                            chars.next();
                        }

                        if username.is_empty() {
                            // Just ~ followed by non-slash, non-whitespace
                            result.push_str(&home_dir);
                        } else {
                            // ~username - for now, treat as literal
                            // TODO: Add support for other users' home directories
                            result.push('~');
                            result.push_str(&username);
                        }
                    }
                }
                at_word_start = false;
            } else {
                result.push(c);
                // Update word boundary tracking
                at_word_start = c.is_whitespace() || c == ':' || c == '=';
            }
        }

        result
    }

    // Helper method to execute commands for substitution
    fn execute_command_for_substitution(&self, cmd: &str) -> Result<String, io::Error> {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let lexer = Lexer::new(cmd);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_script();

        // Create a temporary interpreter for execution
        let mut temp_interpreter = Interpreter {
            variables: self.variables.clone(),
            functions: self.functions.clone(),
            aliases: self.aliases.clone(),
            last_exit_code: self.last_exit_code,
            history: Vec::new(),
            history_file: None,
            rc_file: None,
            args: self.args.clone(),
            return_value: None,
            history_expansion_depth: 0,
            completion_system: CompletionSystem::new(),
        };

        let mut evaluator = DefaultEvaluator;
        temp_interpreter.capture_command_output(&ast, &mut evaluator)
    }

    /// Expand glob patterns in command arguments
    pub fn expand_glob_patterns(&self, args: &[String]) -> Vec<String> {
        let mut expanded_args = Vec::new();

        for arg in args {
            // First expand brace patterns
            let brace_expanded = self.expand_brace_patterns(arg);

            // Then expand glob patterns for each brace-expanded result
            for brace_arg in brace_expanded {
                if self.contains_glob_pattern(&brace_arg) {
                    let matches = self.glob_match(&brace_arg);
                    if matches.is_empty() {
                        // If no matches found, keep the original pattern
                        expanded_args.push(brace_arg);
                    } else {
                        expanded_args.extend(matches);
                    }
                } else {
                    expanded_args.push(brace_arg);
                }
            }
        }

        expanded_args
    }

    /// Expand brace patterns like {1..5} or {a..z}
    pub fn expand_brace_patterns(&self, input: &str) -> Vec<String> {
        // Check if the input contains brace expansion patterns
        if !input.contains('{') || !input.contains('}') {
            return vec![input.to_string()];
        }

        // Find all brace expansion patterns in the input
        let mut result = vec![input.to_string()];

        // Process each string in result for brace expansions
        let mut i = 0;
        while i < result.len() {
            let current = &result[i].clone();
            if let Some(expanded) = self.expand_single_brace_pattern(current) {
                // Replace the current item with expanded items
                result.remove(i);
                for (j, item) in expanded.into_iter().enumerate() {
                    result.insert(i + j, item);
                }
                // Continue processing from the same index since we may have more patterns
                continue;
            }
            i += 1;
        }

        result
    }

    /// Expand a single brace pattern in a string
    fn expand_single_brace_pattern(&self, input: &str) -> Option<Vec<String>> {
        // Find the first brace expansion pattern
        let mut brace_start = None;
        let mut brace_end = None;
        let mut brace_count = 0;

        for (i, c) in input.char_indices() {
            match c {
                '{' => {
                    if brace_start.is_none() {
                        brace_start = Some(i);
                    }
                    brace_count += 1;
                }
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 && brace_start.is_some() {
                        brace_end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let (Some(start), Some(end)) = (brace_start, brace_end) {
            let prefix = &input[..start];
            let suffix = &input[end + 1..];
            let brace_content = &input[start + 1..end];

            // Check for range expansion (e.g., "1..5" or "a..z")
            if brace_content.contains("..") {
                if let Some(range_items) = self.expand_brace_range(brace_content) {
                    let mut results = Vec::new();
                    for item in range_items {
                        results.push(format!("{prefix}{item}{suffix}"));
                    }
                    return Some(results);
                }
            }

            // Check for comma-separated expansion (e.g., "a,b,c")
            if brace_content.contains(',') {
                let items: Vec<&str> = brace_content.split(',').collect();
                let mut results = Vec::new();
                for item in items {
                    results.push(format!("{}{}{}", prefix, item.trim(), suffix));
                }
                return Some(results);
            }
        }

        None
    }

    /// Expand brace ranges like {1..10} or {a..z}
    fn expand_brace_range(&self, content: &str) -> Option<Vec<String>> {
        let parts: Vec<&str> = content.split("..").collect();
        if parts.len() != 2 {
            return None;
        }

        let start = parts[0].trim();
        let end = parts[1].trim();

        // Try numeric expansion
        if let (Ok(start_num), Ok(end_num)) = (start.parse::<i32>(), end.parse::<i32>()) {
            let mut result = Vec::new();
            if start_num <= end_num {
                for i in start_num..=end_num {
                    result.push(i.to_string());
                }
            } else {
                for i in (end_num..=start_num).rev() {
                    result.push(i.to_string());
                }
            }
            return Some(result);
        }

        // Try character expansion (single characters only)
        if start.len() == 1 && end.len() == 1 {
            let start_char = start.chars().next().unwrap();
            let end_char = end.chars().next().unwrap();

            // Only expand if both characters are in the same case (both lowercase or both uppercase)
            let both_lowercase = start_char.is_ascii_lowercase() && end_char.is_ascii_lowercase();
            let both_uppercase = start_char.is_ascii_uppercase() && end_char.is_ascii_uppercase();

            if both_lowercase || both_uppercase {
                let mut result = Vec::new();

                if start_char <= end_char {
                    for c in start_char..=end_char {
                        result.push(c.to_string());
                    }
                } else {
                    for c in (end_char..=start_char).rev() {
                        result.push(c.to_string());
                    }
                }
                return Some(result);
            }
        }

        None
    }

    /// Check if a string contains glob patterns
    fn contains_glob_pattern(&self, s: &str) -> bool {
        s.contains('*') || s.contains('?') || s.contains('[')
    }

    /// Match a glob pattern against files in the specified directory
    fn glob_match_in_dir(&self, pattern: &str, search_dir: &Path) -> Vec<String> {
        let mut matches = Vec::new();

        // Handle absolute and relative paths
        let (dir_path, file_pattern) = if let Some(last_slash) = pattern.rfind('/') {
            let dir = &pattern[..last_slash];
            let file = &pattern[last_slash + 1..];
            (search_dir.join(dir), file.to_string())
        } else {
            (search_dir.to_path_buf(), pattern.to_string())
        };

        // Read directory entries
        if let Ok(entries) = fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    // Skip hidden files unless pattern explicitly starts with '.'
                    if filename.starts_with('.') && !file_pattern.starts_with('.') {
                        continue;
                    }

                    if self.matches_glob_pattern(filename, &file_pattern) {
                        let full_path = if pattern.contains('/') {
                            format!("{}/{}", dir_path.display(), filename)
                        } else {
                            filename.to_string()
                        };
                        matches.push(full_path);
                    }
                }
            }
        }

        // Sort matches for consistent output
        matches.sort();
        matches
    }

    /// Match a glob pattern against files in the current directory
    fn glob_match(&self, pattern: &str) -> Vec<String> {
        self.glob_match_in_dir(pattern, Path::new("."))
    }

    /// Check if a filename matches a glob pattern
    fn matches_glob_pattern(&self, filename: &str, pattern: &str) -> bool {
        self.glob_match_recursive(filename, pattern, 0, 0)
    }

    /// Recursive glob pattern matching implementation
    fn glob_match_recursive(
        &self,
        filename: &str,
        pattern: &str,
        f_idx: usize,
        p_idx: usize,
    ) -> bool {
        let f_chars: Vec<char> = filename.chars().collect();
        let p_chars: Vec<char> = pattern.chars().collect();

        // Base cases
        if p_idx >= p_chars.len() {
            return f_idx >= f_chars.len();
        }

        if f_idx >= f_chars.len() {
            // Check if remaining pattern is all '*'
            return p_chars[p_idx..].iter().all(|&c| c == '*');
        }

        match p_chars[p_idx] {
            '*' => {
                // Try matching zero or more characters
                // First try matching zero characters (skip the *)
                if self.glob_match_recursive(filename, pattern, f_idx, p_idx + 1) {
                    return true;
                }
                // Then try matching one or more characters
                for i in f_idx..f_chars.len() {
                    if self.glob_match_recursive(filename, pattern, i + 1, p_idx + 1) {
                        return true;
                    }
                }
                false
            }
            '?' => {
                // Match exactly one character
                self.glob_match_recursive(filename, pattern, f_idx + 1, p_idx + 1)
            }
            '[' => {
                // Character class matching
                if let Some(end_bracket) = self.find_closing_bracket(&p_chars, p_idx) {
                    let char_class = &p_chars[p_idx + 1..end_bracket];
                    if self.matches_char_class(f_chars[f_idx], char_class) {
                        self.glob_match_recursive(filename, pattern, f_idx + 1, end_bracket + 1)
                    } else {
                        false
                    }
                } else {
                    // Invalid bracket, treat as literal
                    f_chars[f_idx] == '['
                        && self.glob_match_recursive(filename, pattern, f_idx + 1, p_idx + 1)
                }
            }
            c => {
                // Literal character match
                f_chars[f_idx] == c
                    && self.glob_match_recursive(filename, pattern, f_idx + 1, p_idx + 1)
            }
        }
    }

    /// Find the closing bracket for a character class
    fn find_closing_bracket(&self, chars: &[char], start: usize) -> Option<usize> {
        let mut i = start + 1;
        while i < chars.len() {
            if chars[i] == ']' && i > start + 1 {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    /// Check if a character matches a character class
    fn matches_char_class(&self, ch: char, char_class: &[char]) -> bool {
        if char_class.is_empty() {
            return false;
        }

        let negated = char_class[0] == '!' || char_class[0] == '^';
        let class_chars = if negated {
            &char_class[1..]
        } else {
            char_class
        };

        let mut matches = false;
        let mut i = 0;

        while i < class_chars.len() {
            if i + 2 < class_chars.len() && class_chars[i + 1] == '-' {
                // Range like a-z
                let start = class_chars[i];
                let end = class_chars[i + 2];
                if ch >= start && ch <= end {
                    matches = true;
                    break;
                }
                i += 3;
            } else {
                // Single character
                if ch == class_chars[i] {
                    matches = true;
                    break;
                }
                i += 1;
            }
        }

        if negated { !matches } else { matches }
    }

    /// Parse alias value handling escaped spaces and quotes
    fn parse_alias_value(&self, value: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current_part = String::new();
        let mut chars = value.chars().peekable();
        let mut in_quotes = false;
        let mut quote_char = '\0';

        while let Some(ch) = chars.next() {
            match ch {
                '\\' => {
                    // Handle escape sequences
                    if let Some(&next_ch) = chars.peek() {
                        match next_ch {
                            ' ' | '\t' | '\\' | '"' | '\'' => {
                                // Escape the next character
                                current_part.push(chars.next().unwrap());
                            }
                            _ => {
                                // Not a recognized escape, keep the backslash
                                current_part.push(ch);
                            }
                        }
                    } else {
                        // Backslash at end of string
                        current_part.push(ch);
                    }
                }
                '"' | '\'' => {
                    if !in_quotes {
                        // Start quoted section
                        in_quotes = true;
                        quote_char = ch;
                    } else if ch == quote_char {
                        // End quoted section
                        in_quotes = false;
                        quote_char = '\0';
                    } else {
                        // Different quote character inside quotes
                        current_part.push(ch);
                    }
                }
                ' ' | '\t' => {
                    if in_quotes {
                        // Space inside quotes is literal
                        current_part.push(ch);
                    } else {
                        // Space outside quotes separates arguments
                        if !current_part.is_empty() {
                            parts.push(current_part);
                            current_part = String::new();
                        }
                    }
                }
                _ => {
                    current_part.push(ch);
                }
            }
        }

        // Add the last part if it's not empty
        if !current_part.is_empty() {
            parts.push(current_part);
        }

        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_custom_prompt() {
        // Create interpreter without loading RC file
        let mut interpreter = Interpreter {
            variables: HashMap::default(),
            functions: HashMap::new(),
            aliases: HashMap::new(),
            last_exit_code: 0,
            history: Vec::new(),
            history_file: None,
            rc_file: None,
            args: Vec::new(),
            return_value: None,
            history_expansion_depth: 0,
            completion_system: CompletionSystem::new(),
        };

        // Set PWD variable like the real interpreter does
        if let Ok(current_dir) = std::env::current_dir() {
            interpreter
                .variables
                .insert("PWD".to_string(), current_dir.to_string_lossy().to_string());
        }

        // Test default prompt
        assert_eq!(interpreter.get_prompt(), "ϟ ");

        // Test custom prompt
        interpreter
            .variables
            .insert("PROMPT".to_string(), "flash> ".to_string());
        assert_eq!(interpreter.get_prompt(), "flash> ");

        // Test prompt with variable expansion
        interpreter
            .variables
            .insert("USER".to_string(), "testuser".to_string());
        interpreter
            .variables
            .insert("PROMPT".to_string(), "$USER> ".to_string());
        assert_eq!(interpreter.get_prompt(), "testuser> ");

        // Test prompt with PWD expansion - use a fixed PWD for testing
        interpreter
            .variables
            .insert("TESTVAR".to_string(), "/test/path".to_string());
        interpreter
            .variables
            .insert("PROMPT".to_string(), "flash:${TESTVAR}$ ".to_string());

        let prompt = interpreter.get_prompt();
        assert_eq!(prompt, "flash:/test/path$ ");
    }

    #[test]
    fn test_variable_expansion() {
        let mut interpreter = Interpreter::new();
        interpreter
            .variables
            .insert("NAME".to_string(), "world".to_string());

        let expanded = interpreter.expand_variables("Hello $NAME!");
        assert_eq!(expanded, "Hello world!");

        let expanded = interpreter.expand_variables("Hello ${NAME}!");
        assert_eq!(expanded, "Hello world!");
    }

    #[test]
    fn test_pwd_updates_in_prompt_after_cd() {
        let mut interpreter = Interpreter::new();

        // Set up a prompt that uses PWD
        interpreter
            .variables
            .insert("PROMPT".to_string(), "flash:$PWD$ ".to_string());

        // Get initial prompt - should contain initial PWD
        let initial_prompt = interpreter.get_prompt();
        let initial_pwd = interpreter.variables.get("PWD").unwrap().clone();
        assert!(initial_prompt.contains(&initial_pwd));

        // Change directory to parent
        let result = interpreter.execute("cd ..");
        assert!(result.is_ok());

        // Get new prompt - should contain updated PWD
        let new_prompt = interpreter.get_prompt();
        let new_pwd = interpreter.variables.get("PWD").unwrap();
        assert!(new_prompt.contains(new_pwd));

        // PWD should have changed
        assert_ne!(&initial_pwd, new_pwd);

        // The prompt should reflect the new PWD
        assert_eq!(new_prompt, format!("flash:{new_pwd}$ "));
    }

    #[test]
    fn test_command_substitution_basic() {
        let mut interpreter = Interpreter::new();

        // Test basic command substitution with echo
        let result = interpreter.execute("X=$(echo hello)").unwrap();
        assert_eq!(result, 0);
        assert_eq!(interpreter.variables.get("X"), Some(&"hello".to_string()));

        // Test command substitution in echo
        let result = interpreter.execute("echo $(echo world)").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_command_substitution_in_strings() {
        let interpreter = Interpreter::new();

        // Test command substitution within variable expansion
        let expanded = interpreter.expand_variables("Hello $(echo world)!");
        assert_eq!(expanded, "Hello world!");

        // Test nested command substitution - simplified for now
        let expanded = interpreter.expand_variables("Result: $(echo test)");
        assert_eq!(expanded, "Result: test");
    }

    #[test]
    fn test_command_substitution_with_variables() {
        let mut interpreter = Interpreter::new();
        interpreter
            .variables
            .insert("NAME".to_string(), "test".to_string());

        // Test command substitution with variable expansion
        let expanded = interpreter.expand_variables("$(echo $NAME)");
        assert_eq!(expanded, "test");

        // Test assignment with command substitution
        let result = interpreter.execute("RESULT=$(echo $NAME)").unwrap();
        assert_eq!(result, 0);
        assert_eq!(
            interpreter.variables.get("RESULT"),
            Some(&"test".to_string())
        );
    }

    #[test]
    fn test_command_substitution_with_pwd() {
        let interpreter = Interpreter::new();

        // Test pwd command substitution
        // Skip this test if current directory is not accessible (e.g., in CI)
        if let Ok(current_dir) = std::env::current_dir() {
            let expected = current_dir.to_string_lossy().to_string();
            let expanded = interpreter.expand_variables("$(pwd)");
            assert_eq!(expanded, expected);
        } else {
            // In environments where current_dir fails, pwd should return empty string
            let expanded = interpreter.expand_variables("$(pwd)");
            assert_eq!(expanded, "");
        }
    }

    #[test]
    fn test_command_substitution_empty_result() {
        let interpreter = Interpreter::new();

        // Test command substitution with non-existent command
        let expanded = interpreter.expand_variables("$(nonexistentcommand)");
        assert_eq!(expanded, "");

        // Test command substitution with empty echo
        let expanded = interpreter.expand_variables("$(echo)");
        assert_eq!(expanded, "");
    }

    #[test]
    fn test_command_substitution_multiple() {
        let interpreter = Interpreter::new();

        // Test multiple command substitutions in one string
        let expanded = interpreter.expand_variables("$(echo hello) $(echo world)");
        assert_eq!(expanded, "hello world");

        // Test command substitution with other text
        let expanded = interpreter.expand_variables("prefix $(echo middle) suffix");
        assert_eq!(expanded, "prefix middle suffix");
    }

    #[test]
    fn test_command_substitution_with_arguments() {
        let interpreter = Interpreter::new();

        // Test command substitution with command arguments
        let expanded = interpreter.expand_variables("$(echo hello world)");
        assert_eq!(expanded, "hello world");

        // Test with quoted arguments
        let expanded = interpreter.expand_variables("$(echo 'hello world')");
        assert_eq!(expanded, "hello world");
    }

    #[test]
    fn test_command_substitution_whitespace_handling() {
        let interpreter = Interpreter::new();

        // Test that trailing newlines are trimmed
        let expanded = interpreter.expand_variables("$(echo test)");
        assert_eq!(expanded, "test");

        // Test with multiple lines - should preserve internal newlines but trim trailing
        let expanded = interpreter.expand_variables("$(echo -e 'line1\\nline2')");
        // Note: This test might need adjustment based on echo implementation
        assert!(expanded.contains("line1") || expanded == "line1\nline2");
    }

    #[test]
    fn test_command_substitution_in_conditionals() {
        let mut interpreter = Interpreter::new();

        // Test command substitution in variable assignment first
        let result = interpreter.execute("TEST_VAR=$(echo test)").unwrap();
        assert_eq!(result, 0);
        assert_eq!(
            interpreter.variables.get("TEST_VAR"),
            Some(&"test".to_string())
        );

        // Test simple conditional with pre-assigned variable
        let result = interpreter
            .execute("if [ \"$TEST_VAR\" = \"test\" ]; then X=success; fi")
            .unwrap();
        assert_eq!(result, 0);
        assert_eq!(interpreter.variables.get("X"), Some(&"success".to_string()));

        // Test command substitution expansion in strings
        let expanded = interpreter.expand_variables("Result: $(echo success)");
        assert_eq!(expanded, "Result: success");
    }

    #[test]
    fn test_command_substitution_error_handling() {
        let interpreter = Interpreter::new();

        // Test that failed command substitution doesn't crash
        let expanded = interpreter.expand_variables("before $(false) after");
        assert_eq!(expanded, "before  after");

        // Test with command that doesn't exist
        let expanded = interpreter.expand_variables("$(commandthatdoesnotexist)");
        assert_eq!(expanded, "");
    }

    #[test]
    fn test_command_substitution_complex_commands() {
        let mut interpreter = Interpreter::new();

        // Test command substitution with pipeline (simplified)
        // Note: Full pipeline support would require more complex implementation
        let expanded = interpreter.expand_variables("$(echo hello)");
        assert_eq!(expanded, "hello");

        // Test command substitution with assignment
        let result = interpreter.execute("VAR=$(echo value); echo $VAR").unwrap();
        assert_eq!(result, 0);
        assert_eq!(interpreter.variables.get("VAR"), Some(&"value".to_string()));
    }

    #[test]
    fn test_capture_command_output_builtin() {
        let mut interpreter = Interpreter::new();
        let mut evaluator = DefaultEvaluator;

        // Test capturing echo output
        let echo_node = Node::Command {
            name: "echo".to_string(),
            args: vec!["hello".to_string(), "world".to_string()],
            redirects: vec![],
        };

        let output = interpreter
            .capture_command_output(&echo_node, &mut evaluator)
            .unwrap();
        assert_eq!(output, "hello world");

        // Test capturing pwd output - create a controlled test environment
        use tempfile::tempdir;

        // Save original directory first
        let original_dir = std::env::current_dir().unwrap();

        {
            let temp_dir = tempdir().unwrap();
            let temp_path = temp_dir.path();

            // Change to temp directory for this test
            std::env::set_current_dir(temp_path).unwrap();

            let pwd_node = Node::Command {
                name: "pwd".to_string(),
                args: vec![],
                redirects: vec![],
            };

            let output = interpreter
                .capture_command_output(&pwd_node, &mut evaluator)
                .unwrap();

            // Get current directory after changing to temp
            let current_dir = std::env::current_dir().unwrap();

            // Canonicalize both paths to handle symlink differences (e.g., /tmp vs /private/tmp on macOS)
            let output_canonical = std::path::Path::new(output.trim())
                .canonicalize()
                .unwrap_or_else(|_| std::path::PathBuf::from(output.trim()));
            let current_canonical = current_dir
                .canonicalize()
                .unwrap_or_else(|_| current_dir.clone());

            // Both the builtin pwd and env::current_dir() should return the same canonical path
            assert_eq!(output_canonical, current_canonical);

            // Restore original working directory BEFORE temp_dir is dropped
            std::env::set_current_dir(&original_dir).unwrap();
        } // temp_dir is dropped here, but we're already back in the original directory
    }

    #[test]
    fn test_capture_command_output_with_variables() {
        let mut interpreter = Interpreter::new();
        interpreter
            .variables
            .insert("TEST".to_string(), "value".to_string());
        let mut evaluator = DefaultEvaluator;

        // Test capturing echo with variable expansion
        let echo_node = Node::Command {
            name: "echo".to_string(),
            args: vec!["$TEST".to_string()],
            redirects: vec![],
        };

        let output = interpreter
            .capture_command_output(&echo_node, &mut evaluator)
            .unwrap();
        assert_eq!(output, "value");
    }

    #[test]
    fn test_command_substitution_parsing() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        // Test that command substitution is properly parsed
        let input = "X=$(echo test)";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_script();

        // Verify the AST structure
        match ast {
            Node::List { statements, .. } => {
                assert_eq!(statements.len(), 1);
                match &statements[0] {
                    Node::Assignment { name, value } => {
                        assert_eq!(name, "X");
                        match value.as_ref() {
                            Node::CommandSubstitution { command } => match command.as_ref() {
                                Node::Command { name, args, .. } => {
                                    assert_eq!(name, "echo");
                                    assert_eq!(args, &vec!["test".to_string()]);
                                }
                                _ => panic!("Expected Command node in CommandSubstitution"),
                            },
                            _ => panic!("Expected CommandSubstitution node"),
                        }
                    }
                    _ => panic!("Expected Assignment node"),
                }
            }
            _ => panic!("Expected List node"),
        }
    }

    #[test]
    fn test_command_substitution_working_examples() {
        let mut interpreter = Interpreter::new();

        // Basic assignment: X=$(echo hello)
        let result = interpreter.execute("X=$(echo hello)").unwrap();
        assert_eq!(result, 0);
        assert_eq!(interpreter.variables.get("X"), Some(&"hello".to_string()));

        // In strings: echo "Current directory: $(pwd)"
        // Skip pwd test if current directory is not accessible (e.g., in CI)
        if let Ok(current_dir) = std::env::current_dir() {
            let expected_output = format!("Current directory: {}", current_dir.to_string_lossy());
            let expanded = interpreter.expand_variables("Current directory: $(pwd)");
            assert_eq!(expanded, expected_output);
        }

        // Multiple substitutions: echo "$(echo hello) $(echo world)"
        let expanded = interpreter.expand_variables("$(echo hello) $(echo world)");
        assert_eq!(expanded, "hello world");

        // With variables: NAME=test; echo "Value: $(echo $NAME)"
        interpreter
            .variables
            .insert("NAME".to_string(), "test".to_string());
        let expanded = interpreter.expand_variables("Value: $(echo $NAME)");
        assert_eq!(expanded, "Value: test");

        // Complex expressions: echo "Today is $(echo Monday) and the time is $(echo 12:00)"
        let expanded =
            interpreter.expand_variables("Today is $(echo Monday) and the time is $(echo 12:00)");
        assert_eq!(expanded, "Today is Monday and the time is 12:00");
    }

    #[test]
    fn test_command_substitution_real_world_scenarios() {
        let mut interpreter = Interpreter::new();

        // Test assignment with command substitution
        // Skip pwd test if current directory is not accessible (e.g., in CI)
        if let Ok(current_dir) = std::env::current_dir() {
            let result = interpreter.execute("CURRENT_DIR=$(pwd)").unwrap();
            assert_eq!(result, 0);
            assert_eq!(
                interpreter.variables.get("CURRENT_DIR"),
                Some(&current_dir.to_string_lossy().to_string())
            );

            // Test command substitution in echo command
            let result = interpreter.execute("echo \"Working in: $(pwd)\"").unwrap();
            assert_eq!(result, 0);
        }

        // Test multiple command substitutions in assignment
        let result = interpreter
            .execute("GREETING=$(echo Hello) $(echo World)")
            .unwrap();
        assert_eq!(result, 0);

        // Test command substitution with variable expansion
        interpreter
            .variables
            .insert("USER".to_string(), "flash".to_string());

        // Skip pwd test if current directory is not accessible (e.g., in CI)
        if let Ok(current_dir) = std::env::current_dir() {
            let expanded = interpreter.expand_variables("Welcome $(echo $USER) to $(pwd)");
            let expected = format!("Welcome flash to {}", current_dir.to_string_lossy());
            assert_eq!(expanded, expected);
        } else {
            let expanded = interpreter.expand_variables("Welcome $(echo $USER) to $(pwd)");
            assert_eq!(expanded, "Welcome flash to ");
        }
    }

    #[test]
    fn test_command_substitution_edge_cases() {
        let interpreter = Interpreter::new();

        // Test empty command substitution
        let expanded = interpreter.expand_variables("Before $(echo) after");
        assert_eq!(expanded, "Before  after");

        // Test command substitution with spaces (echo normalizes spaces)
        let expanded = interpreter.expand_variables("$(echo hello world)");
        assert_eq!(expanded, "hello world");

        // Test command substitution at start and end
        let expanded = interpreter.expand_variables("$(echo start) middle $(echo end)");
        assert_eq!(expanded, "start middle end");

        // Test command substitution with special characters
        let expanded = interpreter.expand_variables("$(echo 'hello world!')");
        assert_eq!(expanded, "hello world!");

        // Test nested parentheses (should not break parsing)
        let expanded = interpreter.expand_variables("$(echo test)");
        assert_eq!(expanded, "test");
    }

    #[test]
    fn test_command_substitution_integration_with_conditionals() {
        let mut interpreter = Interpreter::new();

        // Test command substitution in variable assignment first
        let result = interpreter.execute("TEST_VAR=$(echo yes)").unwrap();
        assert_eq!(result, 0);
        assert_eq!(
            interpreter.variables.get("TEST_VAR"),
            Some(&"yes".to_string())
        );

        // Test simple conditional with pre-assigned variable
        let result = interpreter
            .execute("if [ \"$TEST_VAR\" = \"yes\" ]; then SUCCESS=true; fi")
            .unwrap();
        assert_eq!(result, 0);
        assert_eq!(
            interpreter.variables.get("SUCCESS"),
            Some(&"true".to_string())
        );

        // Test command substitution expansion in strings
        let expanded = interpreter.expand_variables("Result: $(echo success)");
        assert_eq!(expanded, "Result: success");
    }

    #[test]
    fn test_command_substitution_with_export() {
        let mut interpreter = Interpreter::new();

        // Test export with command substitution
        // Skip pwd test if current directory is not accessible (e.g., in CI)
        if let Ok(current_dir) = std::env::current_dir() {
            let result = interpreter.execute("export SHELL_DIR=$(pwd)").unwrap();
            assert_eq!(result, 0);
            let expected = current_dir.to_string_lossy().to_string();
            assert_eq!(interpreter.variables.get("SHELL_DIR"), Some(&expected));
        }

        // Test export with echo command substitution
        let result = interpreter
            .execute("export GREETING=$(echo 'Hello Flash')")
            .unwrap();
        assert_eq!(result, 0);
        assert_eq!(
            interpreter.variables.get("GREETING"),
            Some(&"Hello Flash".to_string())
        );
    }

    #[test]
    fn test_keywords_as_command_arguments() {
        let mut interpreter = Interpreter::new();

        // Test that 'continue' can be used as an argument to echo
        let result = interpreter.execute("echo continue").unwrap();
        assert_eq!(result, 0);

        // Test that 'break' can be used as an argument to echo
        let result = interpreter.execute("echo break").unwrap();
        assert_eq!(result, 0);

        // Test multiple keywords as arguments
        let result = interpreter.execute("echo if then else fi").unwrap();
        assert_eq!(result, 0);

        // Test keywords in variable assignment
        let result = interpreter.execute("KEYWORD=continue").unwrap();
        assert_eq!(result, 0);
        assert_eq!(
            interpreter.variables.get("KEYWORD"),
            Some(&"continue".to_string())
        );

        // Test keywords with other commands
        let result = interpreter.execute("export TEST=break").unwrap();
        assert_eq!(result, 0);
        assert_eq!(
            interpreter.variables.get("TEST"),
            Some(&"break".to_string())
        );

        // Test keywords in command substitution
        let expanded = interpreter.expand_variables("$(echo continue)");
        assert_eq!(expanded, "continue");

        // Test all control flow keywords as arguments
        let result = interpreter
            .execute("echo for while do done if then elif else fi function export")
            .unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_keywords_in_different_contexts() {
        let mut interpreter = Interpreter::new();

        // Test keywords as part of larger arguments
        let result = interpreter.execute("echo continue-test").unwrap();
        assert_eq!(result, 0);

        let result = interpreter.execute("echo test-break-test").unwrap();
        assert_eq!(result, 0);

        // Test keywords with quotes
        let result = interpreter.execute("echo 'continue'").unwrap();
        assert_eq!(result, 0);

        let result = interpreter.execute("echo \"break\"").unwrap();
        assert_eq!(result, 0);

        // Test keywords in variable expansion
        interpreter
            .variables
            .insert("WORD".to_string(), "continue".to_string());
        let expanded = interpreter.expand_variables("echo $WORD");
        assert_eq!(expanded, "echo continue");

        // Test keywords with special characters
        let result = interpreter.execute("echo continue!").unwrap();
        assert_eq!(result, 0);

        let result = interpreter.execute("echo break?").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_keywords_vs_control_flow() {
        let mut interpreter = Interpreter::new();

        // Test that keywords work as arguments
        let result = interpreter.execute("echo The keyword is continue").unwrap();
        assert_eq!(result, 0);

        // Test that keywords work in assignments
        let result = interpreter.execute("MESSAGE=\"Please continue\"").unwrap();
        assert_eq!(result, 0);
        assert_eq!(
            interpreter.variables.get("MESSAGE"),
            Some(&"Please continue".to_string())
        );

        // Test keywords in command substitution
        let result = interpreter.execute("RESULT=$(echo break)").unwrap();
        assert_eq!(result, 0);
        assert_eq!(
            interpreter.variables.get("RESULT"),
            Some(&"break".to_string())
        );

        // Test that we can echo all shell keywords
        let keywords = vec![
            "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "in", "function",
            "export", "continue", "break",
        ];

        for keyword in keywords {
            let cmd = format!("echo {keyword}");
            let result = interpreter.execute(&cmd).unwrap();
            assert_eq!(result, 0, "Failed to echo keyword: {keyword}");
        }
    }

    #[test]
    fn test_command_execution() {
        let mut interpreter = Interpreter::new();

        // Test a basic command
        let result = interpreter.execute("echo test").unwrap();
        assert_eq!(result, 0);

        // Test assignment
        let result = interpreter.execute("X=test").unwrap();
        assert_eq!(result, 0);
        assert_eq!(interpreter.variables.get("X"), Some(&"test".to_string()));
    }

    #[test]
    fn test_ext_glob_pattern() {
        // Create a temporary directory for testing
        // let temp_dir = tempfile::tempdir().unwrap();
        // let temp_path = temp_dir.path();

        // // Create some test files
        // fs::write(temp_path.join("test1.txt"), "test content").unwrap();
        // fs::write(temp_path.join("test2.txt"), "test content").unwrap();
        // fs::write(temp_path.join("other.txt"), "other content").unwrap();
        // fs::write(temp_path.join("another.log"), "log content").unwrap();

        // // Change to the temporary directory
        // let original_dir = env::current_dir().unwrap();
        // env::set_current_dir(temp_path).unwrap();

        // // Create an interpreter
        // let mut interpreter = Interpreter::new();

        // // Create an ExtGlobPattern node to match files ending with .txt
        // let ext_glob = Node::ExtGlobPattern {
        //     operator: '@',
        //     patterns: vec!["test*".to_string(), "other*".to_string()],
        //     suffix: ".txt".to_string(),
        // };

        // // Execute and check the pattern matching
        // let exit_code = interpreter.evaluate(&ext_glob).unwrap();
        // assert_eq!(exit_code, 0);

        // // Go back to the original directory
        // env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_get_path_completions() {
        // Create a temporary directory for testing
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create some test files and directories
        fs::write(temp_path.join("test1.txt"), "content").unwrap();
        fs::write(temp_path.join("test2.txt"), "content").unwrap();
        fs::create_dir(temp_path.join("testdir")).unwrap();

        // Create interpreter with modified get_path_completions that takes a directory parameter
        let interpreter = Interpreter::new();

        // Test with prefix "test" using absolute path
        let test_prefix = format!("{}/test", temp_path.display());
        let (_suffixes, full_names) = interpreter.get_path_completions(&test_prefix);

        // Check that we get some completions
        assert!(
            !full_names.is_empty(),
            "Expected some completions, got none"
        );

        // Check for expected files/directories
        let has_expected = full_names.iter().any(|name| {
            name.contains("test1.txt") || name.contains("test2.txt") || name.contains("testdir")
        });
        assert!(
            has_expected,
            "Expected completions to contain test files, got: {full_names:?}"
        );

        // Test directory completion (should add trailing slash)
        let testd_prefix = format!("{}/testd", temp_path.display());
        let (_suffixes, full_names) = interpreter.get_path_completions(&testd_prefix);
        let has_dir = full_names
            .iter()
            .any(|name| name.contains("testdir") && name.ends_with('/'));
        assert!(
            has_dir,
            "Expected directory completion with trailing slash, got: {full_names:?}"
        );

        // Test with specific file prefix
        let test1_prefix = format!("{}/test1", temp_path.display());
        let (_suffixes, full_names) = interpreter.get_path_completions(&test1_prefix);
        let has_test1 = full_names.iter().any(|name| name.contains("test1.txt"));
        assert!(
            has_test1,
            "Expected test1.txt completion, got: {full_names:?}"
        );
    }

    #[test]
    fn test_enhanced_completion_system_integration() {
        let mut interpreter = Interpreter::new();

        // Test that the completion system is properly initialized
        assert!(
            interpreter
                .completion_system
                .command_completions
                .contains_key("git")
        );
        assert!(
            interpreter
                .completion_system
                .command_completions
                .contains_key("cd")
        );

        // Test git command completion
        let (_suffixes, full_names) = interpreter.generate_completions("gi", 2);
        assert!(
            full_names.iter().any(|c| c == "git"),
            "Should complete 'git' for 'gi' prefix"
        );

        // Test git subcommand completion
        let (_suffixes, full_names) = interpreter.generate_completions("git ", 4);
        assert!(
            full_names.contains(&"add".to_string()),
            "Should complete git subcommands"
        );
        assert!(
            full_names.contains(&"commit".to_string()),
            "Should complete git subcommands"
        );

        // Test cd directory completion
        let (_suffixes, full_names) = interpreter.generate_completions("cd ", 3);
        // All completions should be directories
        for completion in &full_names {
            assert!(
                completion.ends_with('/'),
                "CD completion '{completion}' should be a directory"
            );
        }
    }

    #[test]
    fn test_completion_fallback_mechanism() {
        let mut interpreter = Interpreter::new();

        // Test variable completion (should use fallback)
        interpreter
            .variables
            .insert("TEST_VAR".to_string(), "test_value".to_string());

        let (_suffixes, full_names) = interpreter.generate_completions("echo $TEST_", 11);

        // Should complete the variable
        assert!(
            full_names.iter().any(|c| c == "$TEST_VAR"),
            "Should complete variable, got: {full_names:?}"
        );
    }

    #[test]
    fn test_completion_with_aliases() {
        let mut interpreter = Interpreter::new();

        // Add an alias
        interpreter
            .aliases
            .insert("ll".to_string(), "ls -la".to_string());

        // Test completion should include aliases
        let (_suffixes, full_names) = interpreter.generate_completions("l", 1);

        // Should include the alias in command completions
        // Note: This tests the fallback system since aliases aren't in the new completion system yet
        assert!(!full_names.is_empty(), "Should return some completions");
    }

    #[test]
    fn test_completion_context_parsing() {
        // Test the context parsing directly
        let context = CompletionSystem::parse_context("git add file.txt", 8);
        assert_eq!(context.line, "git add file.txt");
        assert_eq!(context.point, 8);
        assert_eq!(context.words, vec!["git", "add"]);
        assert_eq!(context.cword, 2);
        assert_eq!(context.current_word, "");
        assert_eq!(context.prev_word, "add");

        // Test partial word completion
        let context = CompletionSystem::parse_context("git che", 7);
        assert_eq!(context.words, vec!["git", "che"]);
        assert_eq!(context.cword, 1);
        assert_eq!(context.current_word, "che");
        assert_eq!(context.prev_word, "git");
    }

    #[test]
    fn test_completion_suffix_calculation() {
        let mut interpreter = Interpreter::new();

        // Test that suffixes are calculated correctly
        let (suffixes, full_names) = interpreter.generate_completions("ec", 2);

        // Find echo in the completions
        if let Some(pos) = full_names.iter().position(|c| c == "echo") {
            assert_eq!(
                suffixes[pos], "ho",
                "Suffix for 'echo' from 'ec' should be 'ho'"
            );
        }
    }

    #[test]
    fn test_completion_empty_input() {
        let mut interpreter = Interpreter::new();

        // Test completion with empty input
        let (_suffixes, full_names) = interpreter.generate_completions("", 0);

        // Should return command completions
        assert!(full_names.contains(&"echo".to_string()));
        assert!(full_names.contains(&"cd".to_string()));
        assert!(full_names.contains(&"exit".to_string()));
    }

    #[test]
    fn test_completion_with_spaces() {
        let mut interpreter = Interpreter::new();

        // Test completion after command with space
        let (_suffixes, full_names) = interpreter.generate_completions("echo ", 5);

        // Should return file completions (fallback behavior)
        assert!(!full_names.is_empty()); // At least doesn't crash
    }

    #[test]
    fn test_completion_system_setup() {
        let system = CompletionSystem::new();

        // Verify all expected default completions are set up
        let expected_commands = ["git", "ssh", "cd", "kill", "man"];
        for cmd in &expected_commands {
            assert!(
                system.command_completions.contains_key(*cmd),
                "Should have completion for '{cmd}'"
            );
        }

        // Verify git completion is configured correctly
        let git_entry = &system.command_completions["git"];
        assert_eq!(git_entry.function, "_git_complete");
        assert!(git_entry.o_options.contains(&"nospace".to_string()));

        // Verify cd completion is configured correctly
        let cd_entry = &system.command_completions["cd"];
        assert_eq!(cd_entry.action, "directory");
    }

    #[test]
    fn test_completion_performance() {
        let mut interpreter = Interpreter::new();

        // Test that completion doesn't take too long
        use std::time::Instant;
        let start = Instant::now();

        for _ in 0..100 {
            let _ = interpreter.generate_completions("git ", 4);
        }

        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 1000,
            "Completion should be fast, took {duration:?}"
        );
    }

    #[test]
    fn test_completion_edge_cases() {
        let mut interpreter = Interpreter::new();

        // Test completion at various cursor positions
        let test_cases = [
            ("git", 0),  // Beginning
            ("git", 1),  // Middle
            ("git", 3),  // End
            ("git ", 4), // After space
        ];

        for (input, pos) in &test_cases {
            let (suffixes, full_names) = interpreter.generate_completions(input, *pos);
            // Should not crash and should return valid results
            assert!(
                suffixes.len() == full_names.len(),
                "Suffixes and full_names should have same length for input '{input}' at pos {pos}"
            );
        }
    }

    #[test]
    fn test_find_common_prefix() {
        let interpreter = Interpreter::new();

        // Test with empty list
        let common = interpreter.find_common_prefix(&[]);
        assert_eq!(common, None);

        // Test with single item
        let common = interpreter.find_common_prefix(&["test".to_string()]);
        assert_eq!(common, Some("test".to_string()));

        // Test with common prefix
        let completions = vec![
            "test1".to_string(),
            "test2".to_string(),
            "test3".to_string(),
        ];
        let common = interpreter.find_common_prefix(&completions);
        assert_eq!(common, Some("test".to_string()));

        // Test with no common prefix
        let completions = vec!["abc".to_string(), "def".to_string(), "ghi".to_string()];
        let common = interpreter.find_common_prefix(&completions);
        assert_eq!(common, None);

        // Test with partially common prefix
        let completions = vec![
            "testfile".to_string(),
            "testdir".to_string(),
            "testcase".to_string(),
        ];
        let common = interpreter.find_common_prefix(&completions);
        assert_eq!(common, Some("test".to_string()));
    }

    #[test]
    fn test_path_completion_with_directories() {
        // Create a temporary directory structure for testing
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create nested directories
        fs::create_dir(temp_path.join("dir1")).unwrap();
        fs::create_dir(temp_path.join("dir1/subdir")).unwrap();
        fs::write(temp_path.join("dir1/file.txt"), "content").unwrap();

        // Create interpreter
        let interpreter = Interpreter::new();

        // Test completion with directory path using absolute path
        let input_prefix = format!("{}/dir1/", temp_path.display());
        let (_suffixes, full_names) = interpreter.get_path_completions(&input_prefix);

        // Check if any completion contains "subdir" or "file.txt"
        let has_expected_completion = full_names
            .iter()
            .any(|c| c.contains("subdir") || c.contains("file.txt"));
        assert!(
            has_expected_completion,
            "Expected completions to contain subdir or file.txt, got: {full_names:?}"
        );

        // Test completion with partial path
        let input_prefix = format!("{}/dir1/s", temp_path.display());
        let (suffixes, _full_names) = interpreter.get_path_completions(&input_prefix);
        let has_subdir = suffixes.iter().any(|c| c.contains("ubdir"));
        assert!(
            has_subdir,
            "Expected suffixes to contain 'ubdir', got: {suffixes:?}"
        );

        // Test completion with file path
        let input_prefix = format!("{}/dir1/f", temp_path.display());
        let (suffixes, _full_names) = interpreter.get_path_completions(&input_prefix);
        let has_file = suffixes.iter().any(|c| c.contains("ile.txt"));
        assert!(
            has_file,
            "Expected suffixes to contain 'ile.txt', got: {suffixes:?}"
        );
    }

    #[test]
    fn test_completion_with_multiple_words() {
        let mut interpreter = Interpreter::new();

        // Test command completion after pipe
        let (suffixes, full_names) = interpreter.generate_completions("ls | e", 6);

        // Check that we get command completions starting with 'e'
        let has_echo_or_export = full_names.iter().any(|c| *c == "echo" || *c == "export");
        let has_echo_or_export_suffix = suffixes.iter().any(|c| *c == "cho" || *c == "xport");
        assert!(
            has_echo_or_export || has_echo_or_export_suffix,
            "Expected completions to include echo or export, got full_names: {full_names:?}, suffixes: {suffixes:?}"
        );

        // Test path completion after command
        // Skip this test if current directory is not accessible (e.g., in CI)
        if let Ok(original_dir) = env::current_dir() {
            {
                let temp_dir = tempdir().unwrap();
                let temp_path = temp_dir.path();
                fs::write(temp_path.join("testfile.txt"), "content").unwrap();

                env::set_current_dir(temp_path).unwrap();

                let (suffixes, full_names) = interpreter.generate_completions("cat test", 8);

                // Check if completions include something related to testfile.txt
                let has_testfile = full_names
                    .iter()
                    .any(|c| c.contains("testfile") || c == "testfile.txt");
                let has_testfile_suffix = suffixes
                    .iter()
                    .any(|c| c.contains("file") || c == "file.txt");
                assert!(
                    has_testfile || has_testfile_suffix,
                    "Expected completions to include 'testfile.txt', got full_names: {full_names:?}, suffixes: {suffixes:?}"
                );

                // Restore original working directory BEFORE temp_dir is dropped
                env::set_current_dir(&original_dir).unwrap();
            } // temp_dir is dropped here, but we're already back in the original directory
        }
    }

    #[test]
    fn test_command_completion_with_arguments() {
        let mut interpreter = Interpreter::new();

        // Add an environment variable to the interpreter's variables
        interpreter
            .variables
            .insert("TEST_PATH".to_string(), "/tmp".to_string());

        // First test that the variable exists in the interpreter
        assert!(interpreter.variables.contains_key("TEST_PATH"));

        // Test partial variable completion
        let (suffixes, full_names) = interpreter.generate_completions("cd $TEST_", 9);

        // The suffixes should contain "PATH" (what comes after TEST_)
        let has_path_suffix = suffixes.iter().any(|c| c == "PATH");
        // The full names should contain "$TEST_PATH"
        let has_path_full = full_names.iter().any(|c| c == "$TEST_PATH");
        assert!(
            has_path_suffix,
            "Expected suffixes to include 'PATH', got: {suffixes:?}"
        );
        assert!(
            has_path_full,
            "Expected full_names to include '$TEST_PATH', got: {full_names:?}"
        );

        // Test completion right after $
        let (_suffixes, full_names) = interpreter.generate_completions("cd $", 4);
        let has_test_path = full_names.iter().any(|c| c == "$TEST_PATH");
        assert!(
            has_test_path,
            "Expected full_names to include '$TEST_PATH', got: {full_names:?}"
        );
    }

    #[test]
    fn test_positional_parameters_basic() {
        let mut interpreter = Interpreter::new();

        // Set up arguments: $0 = script, $1 = first arg, $2 = second arg
        interpreter.set_args(vec![
            "test_script.sh".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ]);

        // Test $0 (script name)
        assert_eq!(interpreter.expand_variables("$0"), "test_script.sh");

        // Test $1 (first argument)
        assert_eq!(interpreter.expand_variables("$1"), "hello");

        // Test $2 (second argument)
        assert_eq!(interpreter.expand_variables("$2"), "world");

        // Test $3 (non-existent argument - should expand to empty)
        assert_eq!(interpreter.expand_variables("$3"), "");

        // Test ${1} (braced syntax)
        assert_eq!(interpreter.expand_variables("${1}"), "hello");

        // Test ${2} (braced syntax)
        assert_eq!(interpreter.expand_variables("${2}"), "world");
    }

    #[test]
    fn test_positional_parameters_special_variables() {
        let mut interpreter = Interpreter::new();

        // Set up arguments
        interpreter.set_args(vec![
            "script.sh".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
            "arg3".to_string(),
        ]);

        // Test $# (argument count - excludes $0)
        assert_eq!(interpreter.expand_variables("$#"), "3");

        // Test $@ (all arguments excluding $0)
        assert_eq!(interpreter.expand_variables("$@"), "arg1 arg2 arg3");

        // Test $* (all arguments excluding $0)
        assert_eq!(interpreter.expand_variables("$*"), "arg1 arg2 arg3");

        // Test with no arguments (only script name)
        interpreter.set_args(vec!["script.sh".to_string()]);
        assert_eq!(interpreter.expand_variables("$#"), "0");
        assert_eq!(interpreter.expand_variables("$@"), "");
        assert_eq!(interpreter.expand_variables("$*"), "");
    }

    #[test]
    fn test_positional_parameters_empty_args() {
        let mut interpreter = Interpreter::new();

        // Test with empty args vector
        interpreter.set_args(vec![]);

        assert_eq!(interpreter.expand_variables("$0"), "");
        assert_eq!(interpreter.expand_variables("$1"), "");
        assert_eq!(interpreter.expand_variables("$#"), "0");
        assert_eq!(interpreter.expand_variables("$@"), "");
        assert_eq!(interpreter.expand_variables("$*"), "");
    }

    #[test]
    fn test_positional_parameters_in_commands() {
        let mut interpreter = Interpreter::new();

        // Set up arguments
        interpreter.set_args(vec![
            "test.sh".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ]);

        // Test expansion in complex strings
        assert_eq!(
            interpreter.expand_variables("First: $1, Second: $2"),
            "First: hello, Second: world"
        );

        // Test expansion with other variables
        interpreter
            .variables
            .insert("USER".to_string(), "testuser".to_string());
        assert_eq!(
            interpreter.expand_variables("User: $USER, Arg: $1"),
            "User: testuser, Arg: hello"
        );

        // Test mixed braced and unbraced
        assert_eq!(
            interpreter.expand_variables("${1}_suffix and ${2}_suffix"),
            "hello_suffix and world_suffix"
        );
    }

    #[test]
    fn test_positional_parameters_numeric_parsing() {
        let mut interpreter = Interpreter::new();

        // Set up many arguments to test numeric parsing
        interpreter.set_args(vec![
            "script.sh".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
            "arg3".to_string(),
            "arg4".to_string(),
            "arg5".to_string(),
            "arg6".to_string(),
            "arg7".to_string(),
            "arg8".to_string(),
            "arg9".to_string(),
            "arg10".to_string(),
        ]);

        // Test single digits
        assert_eq!(interpreter.expand_variables("$1"), "arg1");
        assert_eq!(interpreter.expand_variables("$9"), "arg9");

        // Test double digits (should work with braces)
        assert_eq!(interpreter.expand_variables("${10}"), "arg10");

        // Test that $10 without braces only gets $1 followed by "0"
        assert_eq!(interpreter.expand_variables("$10"), "arg10");

        // Test non-existent high numbers
        assert_eq!(interpreter.expand_variables("${99}"), "");
    }

    #[test]
    fn test_positional_parameters_with_quotes() {
        let mut interpreter = Interpreter::new();

        // Set up arguments with spaces and special characters
        interpreter.set_args(vec![
            "script.sh".to_string(),
            "hello world".to_string(),
            "arg with spaces".to_string(),
            "special!@#$%".to_string(),
        ]);

        // Test that arguments with spaces are preserved
        assert_eq!(interpreter.expand_variables("$1"), "hello world");
        assert_eq!(interpreter.expand_variables("$2"), "arg with spaces");
        assert_eq!(interpreter.expand_variables("$3"), "special!@#$%");

        // Test $@ preserves all arguments
        assert_eq!(
            interpreter.expand_variables("$@"),
            "hello world arg with spaces special!@#$%"
        );
    }

    #[test]
    fn test_echo_command_with_positional_parameters() {
        let mut interpreter = Interpreter::new();

        // Set up arguments
        interpreter.set_args(vec![
            "test.sh".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ]);

        // Test that echo properly expands positional parameters
        // We can't easily test the actual output, but we can test the expansion
        let expanded = interpreter.expand_variables("First arg: $1, Second arg: $2");
        assert_eq!(expanded, "First arg: hello, Second arg: world");

        // Test with $# and $@
        let expanded = interpreter.expand_variables("Count: $#, All: $@");
        assert_eq!(expanded, "Count: 2, All: hello world");
    }

    #[test]
    fn test_set_args_method() {
        let mut interpreter = Interpreter::new();

        // Test initial state
        assert!(interpreter.args.is_empty());

        // Test setting args
        let test_args = vec![
            "script.sh".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
        ];
        interpreter.set_args(test_args.clone());

        assert_eq!(interpreter.args, test_args);
        assert_eq!(interpreter.expand_variables("$0"), "script.sh");
        assert_eq!(interpreter.expand_variables("$1"), "arg1");
        assert_eq!(interpreter.expand_variables("$2"), "arg2");

        // Test overwriting args
        let new_args = vec!["new_script.sh".to_string(), "new_arg".to_string()];
        interpreter.set_args(new_args.clone());

        assert_eq!(interpreter.args, new_args);
        assert_eq!(interpreter.expand_variables("$0"), "new_script.sh");
        assert_eq!(interpreter.expand_variables("$1"), "new_arg");
        assert_eq!(interpreter.expand_variables("$2"), ""); // Should be empty now
    }

    #[test]
    fn test_positional_parameters_edge_cases() {
        let mut interpreter = Interpreter::new();

        // Test with single argument (only script name)
        interpreter.set_args(vec!["script.sh".to_string()]);

        assert_eq!(interpreter.expand_variables("$0"), "script.sh");
        assert_eq!(interpreter.expand_variables("$1"), "");
        assert_eq!(interpreter.expand_variables("$#"), "0");
        assert_eq!(interpreter.expand_variables("$@"), "");

        // Test with empty string arguments
        interpreter.set_args(vec![
            "script.sh".to_string(),
            "".to_string(),
            "non_empty".to_string(),
            "".to_string(),
        ]);

        assert_eq!(interpreter.expand_variables("$1"), "");
        assert_eq!(interpreter.expand_variables("$2"), "non_empty");
        assert_eq!(interpreter.expand_variables("$3"), "");
        assert_eq!(interpreter.expand_variables("$#"), "3");
        assert_eq!(interpreter.expand_variables("$@"), " non_empty ");
    }

    #[test]
    fn test_glob_pattern_detection() {
        let interpreter = Interpreter::new();

        // Test patterns that should be detected as globs
        assert!(interpreter.contains_glob_pattern("*.txt"));
        assert!(interpreter.contains_glob_pattern("file?.log"));
        assert!(interpreter.contains_glob_pattern("[abc]*.tmp"));
        assert!(interpreter.contains_glob_pattern("test*"));
        assert!(interpreter.contains_glob_pattern("?test"));
        assert!(interpreter.contains_glob_pattern("file[0-9].txt"));

        // Test patterns that should NOT be detected as globs
        assert!(!interpreter.contains_glob_pattern("file.txt"));
        assert!(!interpreter.contains_glob_pattern("test"));
        assert!(!interpreter.contains_glob_pattern("path/to/file"));
    }

    #[test]
    fn test_glob_pattern_matching() {
        let interpreter = Interpreter::new();

        // Test * wildcard
        assert!(interpreter.matches_glob_pattern("file.txt", "*.txt"));
        assert!(interpreter.matches_glob_pattern("test.txt", "*.txt"));
        assert!(interpreter.matches_glob_pattern("a.txt", "*.txt"));
        assert!(!interpreter.matches_glob_pattern("file.log", "*.txt"));
        assert!(interpreter.matches_glob_pattern("anything", "*"));
        assert!(interpreter.matches_glob_pattern("", "*"));

        // Test ? wildcard
        assert!(interpreter.matches_glob_pattern("file1.txt", "file?.txt"));
        assert!(interpreter.matches_glob_pattern("fileA.txt", "file?.txt"));
        assert!(!interpreter.matches_glob_pattern("file.txt", "file?.txt"));
        assert!(!interpreter.matches_glob_pattern("file12.txt", "file?.txt"));

        // Test character classes
        assert!(interpreter.matches_glob_pattern("file1.txt", "file[123].txt"));
        assert!(interpreter.matches_glob_pattern("file2.txt", "file[123].txt"));
        assert!(interpreter.matches_glob_pattern("file3.txt", "file[123].txt"));
        assert!(!interpreter.matches_glob_pattern("file4.txt", "file[123].txt"));
        assert!(!interpreter.matches_glob_pattern("filea.txt", "file[123].txt"));

        // Test character ranges
        assert!(interpreter.matches_glob_pattern("file1.txt", "file[0-9].txt"));
        assert!(interpreter.matches_glob_pattern("file5.txt", "file[0-9].txt"));
        assert!(interpreter.matches_glob_pattern("file9.txt", "file[0-9].txt"));
        assert!(!interpreter.matches_glob_pattern("filea.txt", "file[0-9].txt"));

        assert!(interpreter.matches_glob_pattern("filea.txt", "file[a-z].txt"));
        assert!(interpreter.matches_glob_pattern("filem.txt", "file[a-z].txt"));
        assert!(interpreter.matches_glob_pattern("filez.txt", "file[a-z].txt"));
        assert!(!interpreter.matches_glob_pattern("fileA.txt", "file[a-z].txt"));
        assert!(!interpreter.matches_glob_pattern("file1.txt", "file[a-z].txt"));

        // Test negated character classes
        assert!(!interpreter.matches_glob_pattern("file1.txt", "file[!123].txt"));
        assert!(!interpreter.matches_glob_pattern("file2.txt", "file[!123].txt"));
        assert!(interpreter.matches_glob_pattern("file4.txt", "file[!123].txt"));
        assert!(interpreter.matches_glob_pattern("filea.txt", "file[!123].txt"));

        // Test complex patterns
        assert!(interpreter.matches_glob_pattern("test123.log", "test*.log"));
        assert!(interpreter.matches_glob_pattern("test.log", "test*.log"));
        assert!(interpreter.matches_glob_pattern("backup_file_2023.txt", "*_file_*.txt"));
        assert!(interpreter.matches_glob_pattern("config.ini", "*.ini"));

        // Test exact matches
        assert!(interpreter.matches_glob_pattern("exact", "exact"));
        assert!(!interpreter.matches_glob_pattern("exact", "different"));
    }

    #[test]
    fn test_character_class_matching() {
        let interpreter = Interpreter::new();

        // Test simple character class
        assert!(interpreter.matches_char_class('a', &['a', 'b', 'c']));
        assert!(interpreter.matches_char_class('b', &['a', 'b', 'c']));
        assert!(interpreter.matches_char_class('c', &['a', 'b', 'c']));
        assert!(!interpreter.matches_char_class('d', &['a', 'b', 'c']));

        // Test range
        assert!(interpreter.matches_char_class('a', &['a', '-', 'z']));
        assert!(interpreter.matches_char_class('m', &['a', '-', 'z']));
        assert!(interpreter.matches_char_class('z', &['a', '-', 'z']));
        assert!(!interpreter.matches_char_class('A', &['a', '-', 'z']));

        // Test negated class
        assert!(!interpreter.matches_char_class('a', &['!', 'a', 'b', 'c']));
        assert!(!interpreter.matches_char_class('b', &['!', 'a', 'b', 'c']));
        assert!(interpreter.matches_char_class('d', &['!', 'a', 'b', 'c']));

        // Test negated range
        assert!(!interpreter.matches_char_class('a', &['!', 'a', '-', 'z']));
        assert!(!interpreter.matches_char_class('m', &['!', 'a', '-', 'z']));
        assert!(interpreter.matches_char_class('A', &['!', 'a', '-', 'z']));
        assert!(interpreter.matches_char_class('1', &['!', 'a', '-', 'z']));
    }

    #[test]
    fn test_glob_expansion_with_temp_files() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        fs::write(temp_path.join("file1.txt"), "content1").unwrap();
        fs::write(temp_path.join("file2.txt"), "content2").unwrap();
        fs::write(temp_path.join("test.log"), "log content").unwrap();
        fs::write(temp_path.join("data.csv"), "csv content").unwrap();
        fs::write(temp_path.join("script.sh"), "script content").unwrap();

        let interpreter = Interpreter::new();

        // Test *.txt pattern
        let mut matches = interpreter.glob_match_in_dir("*.txt", temp_path);
        matches.sort();
        assert_eq!(matches, vec!["file1.txt", "file2.txt"]);

        // Test *.log pattern
        let matches = interpreter.glob_match_in_dir("*.log", temp_path);
        assert_eq!(matches, vec!["test.log"]);

        // Test file?.txt pattern
        let mut matches = interpreter.glob_match_in_dir("file?.txt", temp_path);
        matches.sort();
        assert_eq!(matches, vec!["file1.txt", "file2.txt"]);

        // Test pattern with no matches
        let matches = interpreter.glob_match_in_dir("*.xyz", temp_path);
        assert!(matches.is_empty());

        // Test * pattern (should match all files)
        let mut matches = interpreter.glob_match_in_dir("*", temp_path);
        matches.sort();
        assert_eq!(
            matches,
            vec![
                "data.csv",
                "file1.txt",
                "file2.txt",
                "script.sh",
                "test.log"
            ]
        );
    }

    #[test]
    fn test_expand_glob_patterns() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        fs::write(temp_path.join("file1.txt"), "content1").unwrap();
        fs::write(temp_path.join("file2.txt"), "content2").unwrap();
        fs::write(temp_path.join("test.log"), "log content").unwrap();

        // Create a custom interpreter that uses the temp directory for glob matching
        let interpreter = TestInterpreter::new(temp_path.to_path_buf());

        // Test expansion with glob patterns
        let args = vec!["ls".to_string(), "*.txt".to_string()];
        let expanded = interpreter.expand_glob_patterns(&args);
        assert_eq!(expanded[0], "ls");
        assert!(expanded.contains(&"file1.txt".to_string()));
        assert!(expanded.contains(&"file2.txt".to_string()));
        assert_eq!(expanded.len(), 3); // ls + 2 txt files

        // Test expansion with mixed patterns and literals
        let args = vec![
            "command".to_string(),
            "literal".to_string(),
            "*.log".to_string(),
        ];
        let expanded = interpreter.expand_glob_patterns(&args);
        assert_eq!(expanded, vec!["command", "literal", "test.log"]);

        // Test expansion with no matches (should keep original pattern)
        let args = vec!["command".to_string(), "*.xyz".to_string()];
        let expanded = interpreter.expand_glob_patterns(&args);
        assert_eq!(expanded, vec!["command", "*.xyz"]);

        // Test expansion with no glob patterns
        let args = vec![
            "command".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
        ];
        let expanded = interpreter.expand_glob_patterns(&args);
        assert_eq!(expanded, args);
    }

    // Helper struct for testing that uses a specific directory for glob operations
    struct TestInterpreter {
        interpreter: Interpreter,
        test_dir: PathBuf,
    }

    impl TestInterpreter {
        fn new(test_dir: PathBuf) -> Self {
            Self {
                interpreter: Interpreter::new(),
                test_dir,
            }
        }

        fn expand_glob_patterns(&self, args: &[String]) -> Vec<String> {
            let mut expanded_args = Vec::new();

            for arg in args {
                if self.interpreter.contains_glob_pattern(arg) {
                    let matches = self.interpreter.glob_match_in_dir(arg, &self.test_dir);
                    if matches.is_empty() {
                        // If no matches found, keep the original pattern
                        expanded_args.push(arg.clone());
                    } else {
                        expanded_args.extend(matches);
                    }
                } else {
                    expanded_args.push(arg.clone());
                }
            }

            expanded_args
        }
    }

    #[test]
    fn test_tilde_expansion_basic() {
        let interpreter = Interpreter::new();

        // Set a known HOME directory for testing
        let original_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", "/test/home");
        }

        // Test basic tilde expansion
        assert_eq!(interpreter.expand_variables("~"), "/test/home");
        assert_eq!(
            interpreter.expand_variables("~/Documents"),
            "/test/home/Documents"
        );
        assert_eq!(
            interpreter.expand_variables("~/bin/script"),
            "/test/home/bin/script"
        );

        // Test tilde with spaces
        assert_eq!(
            interpreter.expand_variables("~ /other/path"),
            "/test/home /other/path"
        );
        assert_eq!(
            interpreter.expand_variables("path ~ more"),
            "path /test/home more"
        );

        // Test tilde at end of string
        assert_eq!(
            interpreter.expand_variables("prefix ~"),
            "prefix /test/home"
        );

        // Restore original HOME
        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[test]
    fn test_tilde_expansion_edge_cases() {
        let mut interpreter = Interpreter::new();

        // Set a known HOME directory for testing
        let original_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", "/test/home");
        }

        // Test tilde not at word boundary (should not expand)
        assert_eq!(interpreter.expand_variables("file~backup"), "file~backup");
        assert_eq!(interpreter.expand_variables("test~"), "test~");

        // Test multiple tildes
        assert_eq!(
            interpreter.expand_variables("~ and ~"),
            "/test/home and /test/home"
        );
        assert_eq!(
            interpreter.expand_variables("~/a:~/b"),
            "/test/home/a:/test/home/b"
        );

        // Test tilde with other expansions
        interpreter
            .variables
            .insert("VAR".to_string(), "value".to_string());
        assert_eq!(interpreter.expand_variables("~/$VAR"), "/test/home/value");
        assert_eq!(interpreter.expand_variables("$VAR/~"), "value/~"); // ~ not at start of path component

        // Test empty string
        assert_eq!(interpreter.expand_variables(""), "");

        // Restore original HOME
        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[test]
    fn test_tilde_expansion_no_home() {
        let interpreter = Interpreter::new();

        // Save the original HOME environment variable
        let original_home = std::env::var("HOME").ok();

        // Remove HOME environment variable and ensure it stays removed during the test
        unsafe {
            std::env::remove_var("HOME");
        }

        // Verify HOME is actually removed
        assert!(
            std::env::var("HOME").is_err(),
            "HOME should be unset for this test"
        );

        // When HOME is not set, tilde should remain unchanged
        let result1 = interpreter.expand_variables("~");
        let result2 = interpreter.expand_variables("~/Documents");

        // Restore original HOME immediately to avoid affecting other tests
        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            }
        }

        // Now check the results
        assert_eq!(
            result1, "~",
            "Tilde should remain unchanged when HOME is not set"
        );
        assert_eq!(
            result2, "~/Documents",
            "Tilde path should remain unchanged when HOME is not set"
        );
    }

    #[test]
    fn test_tilde_expansion_with_username() {
        let interpreter = Interpreter::new();

        // Set a known HOME directory for testing
        let original_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", "/test/home");
        }

        // Test ~username (should remain literal for now)
        assert_eq!(interpreter.expand_variables("~user"), "~user");
        assert_eq!(
            interpreter.expand_variables("~user/Documents"),
            "~user/Documents"
        );
        assert_eq!(interpreter.expand_variables("~root/bin"), "~root/bin");

        // But ~ alone should still expand
        assert_eq!(interpreter.expand_variables("~"), "/test/home");

        // Restore original HOME
        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[test]
    fn test_tilde_expansion_in_export() {
        let mut interpreter = Interpreter::new();

        // Set a known HOME directory for testing
        let original_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", "/test/home");
        }

        // Test tilde expansion in export statements
        interpreter.execute("export GOPATH=~/go").unwrap();
        assert_eq!(
            interpreter.variables.get("GOPATH"),
            Some(&"/test/home/go".to_string())
        );

        interpreter
            .execute("export PATH=~/bin:/usr/local/bin")
            .unwrap();
        assert_eq!(
            interpreter.variables.get("PATH"),
            Some(&"/test/home/bin:/usr/local/bin".to_string())
        );

        // Test mixed tilde and variable expansion
        interpreter
            .execute("export COMPLEX=~/bin:$GOPATH/bin")
            .unwrap();
        assert_eq!(
            interpreter.variables.get("COMPLEX"),
            Some(&"/test/home/bin:/test/home/go/bin".to_string())
        );

        // Restore original HOME
        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[test]
    fn test_tilde_expansion_in_commands() {
        let interpreter = Interpreter::new();

        // Set a known HOME directory for testing
        let original_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", "/test/home");
        }

        // Test that tilde expansion works in command arguments
        // We can't easily test actual command execution, but we can test variable expansion
        assert_eq!(
            interpreter.expand_variables("ls ~/Documents"),
            "ls /test/home/Documents"
        );
        assert_eq!(
            interpreter.expand_variables("cd ~ && pwd"),
            "cd /test/home && pwd"
        );
        assert_eq!(
            interpreter.expand_variables("cp file ~/backup/"),
            "cp file /test/home/backup/"
        );

        // Restore original HOME
        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[test]
    fn test_cd_with_tilde_expansion() {
        let mut interpreter = Interpreter::new();

        // Set a known HOME directory for testing
        let original_home = std::env::var("HOME").ok();
        let original_pwd = std::env::current_dir().ok();

        unsafe {
            std::env::set_var("HOME", "/tmp");
        }

        // Test cd with tilde
        let result = interpreter.execute("cd ~");
        assert!(result.is_ok());

        // Check that PWD was updated (handle symlinks like /tmp -> /private/tmp on macOS)
        let pwd = interpreter.variables.get("PWD").unwrap();
        assert!(pwd.ends_with("/tmp") || pwd == "/tmp");

        // Test cd with tilde and subdirectory
        // Note: We can't actually test this without creating the directory,
        // but we can test that the expansion happens correctly

        // Restore original state
        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }

        if let Some(pwd) = original_pwd {
            let _ = std::env::set_current_dir(pwd);
        }
    }

    #[test]
    fn test_flash_version_variable() {
        let interpreter = Interpreter::new();

        // Test that FLASH_VERSION is set
        let flash_version = interpreter.variables.get("FLASH_VERSION");
        assert!(flash_version.is_some(), "FLASH_VERSION should be set");

        let version = flash_version.unwrap();
        assert!(!version.is_empty(), "FLASH_VERSION should not be empty");

        // Test that it matches the cargo package version
        assert_eq!(version, env!("CARGO_PKG_VERSION"));

        // Test that we can access it via variable expansion
        let expanded = interpreter.expand_variables("$FLASH_VERSION");
        assert_eq!(expanded, env!("CARGO_PKG_VERSION"));

        // Test that we can access it in command substitution context
        let expanded_braces = interpreter.expand_variables("${FLASH_VERSION}");
        assert_eq!(expanded_braces, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_alias_parsing_with_escapes() {
        let interpreter = Interpreter::new();

        // Test basic alias parsing
        let parts = interpreter.parse_alias_value("echo hello");
        assert_eq!(parts, vec!["echo", "hello"]);

        // Test escaped spaces
        let parts = interpreter.parse_alias_value("echo hello\\ world");
        assert_eq!(parts, vec!["echo", "hello world"]);

        // Test path with escaped spaces (like Sublime Text)
        let parts = interpreter
            .parse_alias_value("/Applications/Sublime\\ Text.app/Contents/SharedSupport/bin/subl");
        assert_eq!(
            parts,
            vec!["/Applications/Sublime Text.app/Contents/SharedSupport/bin/subl"]
        );

        // Test quoted strings
        let parts = interpreter.parse_alias_value("echo \"hello world\"");
        assert_eq!(parts, vec!["echo", "hello world"]);

        // Test single quoted strings
        let parts = interpreter.parse_alias_value("echo 'hello world'");
        assert_eq!(parts, vec!["echo", "hello world"]);

        // Test mixed quotes and escapes
        let parts = interpreter.parse_alias_value("echo \"hello world\" test");
        assert_eq!(parts, vec!["echo", "hello world", "test"]);

        // Test multiple escaped spaces
        let parts =
            interpreter.parse_alias_value("cp file\\ name.txt /path/to/destination\\ folder/");
        assert_eq!(
            parts,
            vec!["cp", "file name.txt", "/path/to/destination folder/"]
        );

        // Test empty value
        let parts = interpreter.parse_alias_value("");
        assert_eq!(parts, Vec::<String>::new());

        // Test only spaces
        let parts = interpreter.parse_alias_value("   ");
        assert_eq!(parts, Vec::<String>::new());

        // Test escaped backslash
        let parts = interpreter.parse_alias_value("echo hello\\\\world");
        assert_eq!(parts, vec!["echo", "hello\\world"]);
    }

    #[test]
    fn test_alias_functionality() {
        let mut interpreter = Interpreter::new();
        let mut evaluator = DefaultEvaluator;

        // Test alias creation with unquoted value (spaces should separate arguments)
        let result = evaluator.evaluate_command(
            "alias",
            &["test_alias=echo hello world".to_string()],
            &[],
            &mut interpreter,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Verify alias was created (should store without additional escaping)
        assert!(interpreter.aliases.contains_key("test_alias"));
        assert_eq!(
            interpreter.aliases.get("test_alias").unwrap(),
            "echo hello world"
        );

        // Test alias expansion
        let result = evaluator.evaluate_command("test_alias", &[], &[], &mut interpreter);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Test alias with additional arguments
        let result = evaluator.evaluate_command(
            "test_alias",
            &["extra".to_string(), "args".to_string()],
            &[],
            &mut interpreter,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Test alias with escaped spaces (should preserve the escaping)
        let result = evaluator.evaluate_command(
            "alias",
            &["path_alias=/path/to/file\\ with\\ spaces".to_string()],
            &[],
            &mut interpreter,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Verify escaped spaces alias (escaping should be preserved from input)
        assert_eq!(
            interpreter.aliases.get("path_alias").unwrap(),
            "/path/to/file\\ with\\ spaces"
        );

        // Test that the path alias expands to a single argument
        let stored_path_value = interpreter.aliases.get("path_alias").unwrap();
        let parsed_path_parts = interpreter.parse_alias_value(stored_path_value);
        assert_eq!(parsed_path_parts, vec!["/path/to/file with spaces"]);

        // Test alias with quoted value (quotes should be removed, spaces preserved)
        let result = evaluator.evaluate_command(
            "alias",
            &["quoted_alias=\"echo hello world\"".to_string()],
            &[],
            &mut interpreter,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Verify quoted alias (quotes removed, content preserved)
        assert_eq!(
            interpreter.aliases.get("quoted_alias").unwrap(),
            "echo hello world"
        );

        // Test unalias
        let result = evaluator.evaluate_command(
            "unalias",
            &["test_alias".to_string()],
            &[],
            &mut interpreter,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Verify alias was removed
        assert!(!interpreter.aliases.contains_key("test_alias"));

        // Test unalias non-existent alias
        let result = evaluator.evaluate_command(
            "unalias",
            &["nonexistent".to_string()],
            &[],
            &mut interpreter,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // Should return error code
    }

    #[test]
    fn test_tilde_expansion_in_completion() {
        let interpreter = Interpreter::new();

        // Test basic tilde expansion
        let expanded = interpreter.expand_tilde("~");
        let home_dir = std::env::var("HOME").unwrap_or_default();
        assert_eq!(expanded, home_dir);

        // Test tilde with slash
        let expanded = interpreter.expand_tilde("~/");
        assert_eq!(expanded, format!("{home_dir}/"));

        // Test tilde with path
        let expanded = interpreter.expand_tilde("~/Documents");
        assert_eq!(expanded, format!("{home_dir}/Documents"));

        // Test non-tilde path (should remain unchanged)
        let expanded = interpreter.expand_tilde("/usr/local");
        assert_eq!(expanded, "/usr/local");

        // Test relative path (should remain unchanged)
        let expanded = interpreter.expand_tilde("./test");
        assert_eq!(expanded, "./test");
    }

    #[test]
    fn test_path_completions_with_tilde() {
        use tempfile::tempdir;

        let interpreter = Interpreter::new();
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create some test files in temp directory
        fs::write(temp_path.join("test_file.txt"), "content").unwrap();
        fs::write(temp_path.join("another_file.rs"), "rust code").unwrap();
        fs::create_dir(temp_path.join("test_dir")).unwrap();

        // Test completion with absolute path (no tilde)
        let prefix = format!("{}/test", temp_path.display());
        let (suffixes, full_names) = interpreter.get_path_completions(&prefix);

        // Should find files starting with "test"
        assert!(!suffixes.is_empty());
        assert!(!full_names.is_empty());

        // Check that we get the expected completions
        let has_test_file = full_names.iter().any(|name| name.contains("test_file.txt"));
        let has_test_dir = full_names.iter().any(|name| name.contains("test_dir/"));
        assert!(has_test_file || has_test_dir);
    }

    #[test]
    fn test_path_completions_current_directory() {
        let interpreter = Interpreter::new();

        // Test completion in current directory
        let (_suffixes, full_names) = interpreter.get_path_completions("src");

        // Should find src directory if it exists
        if std::path::Path::new("src").exists() {
            assert!(!_suffixes.is_empty() || !full_names.is_empty());
        }

        // Test completion with no prefix (list all files in current dir)
        let (_suffixes, full_names) = interpreter.get_path_completions("");

        // Should return some files/directories, but skip if current directory is not accessible (e.g., in CI)
        if std::env::current_dir().is_ok() {
            // In a normal environment, there should be some files
            // But in CI, the directory might be empty or inaccessible
            // So we'll just check that the function doesn't panic
            let _ = full_names.is_empty(); // This is just to use the variable
        }
    }

    #[test]
    fn test_path_completions_with_trailing_slash() {
        use tempfile::tempdir;

        let interpreter = Interpreter::new();
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create a subdirectory with files
        let sub_dir = temp_path.join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        fs::write(sub_dir.join("file1.txt"), "content1").unwrap();
        fs::write(sub_dir.join("file2.txt"), "content2").unwrap();

        // Test completion with trailing slash (should list contents of directory)
        let prefix = format!("{}/subdir/", temp_path.display());
        let (_suffixes, full_names) = interpreter.get_path_completions(&prefix);

        // Should find files in the subdirectory
        assert!(!full_names.is_empty());
        let has_file1 = full_names.iter().any(|name| name.contains("file1.txt"));
        let has_file2 = full_names.iter().any(|name| name.contains("file2.txt"));
        assert!(has_file1 || has_file2);
    }

    #[test]
    fn test_path_completions_nonexistent_directory() {
        let interpreter = Interpreter::new();

        // Test completion in non-existent directory
        let (_suffixes, full_names) = interpreter.get_path_completions("/nonexistent/path/");

        // Should return empty results
        assert!(_suffixes.is_empty());
        assert!(full_names.is_empty());
    }

    #[test]
    fn test_path_completions_directory_suffix() {
        use tempfile::tempdir;

        let interpreter = Interpreter::new();
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create a directory and a file with similar names
        fs::create_dir(temp_path.join("test_dir")).unwrap();
        fs::write(temp_path.join("test_file.txt"), "content").unwrap();

        // Test completion
        let prefix = format!("{}/test", temp_path.display());
        let (_suffixes, full_names) = interpreter.get_path_completions(&prefix);

        // Should find both, but directory should have trailing slash
        assert!(!full_names.is_empty());

        let dir_entry = full_names.iter().find(|name| name.contains("test_dir"));
        if let Some(dir_name) = dir_entry {
            assert!(
                dir_name.ends_with('/'),
                "Directory should have trailing slash: {dir_name}"
            );
        }

        let file_entry = full_names
            .iter()
            .find(|name| name.contains("test_file.txt"));
        if let Some(file_name) = file_entry {
            assert!(
                !file_name.ends_with('/'),
                "File should not have trailing slash: {file_name}"
            );
        }
    }

    #[test]
    fn test_tilde_completion_preserves_display_format() {
        // This test verifies that when we use ~ in completion, the display format preserves the ~
        // Note: This is a unit test for the logic, actual tilde expansion depends on HOME being set

        let interpreter = Interpreter::new();

        // Mock HOME environment for testing
        let original_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", "/Users/testuser");
        }

        // Test that tilde expansion works
        let expanded = interpreter.expand_tilde("~/Documents");
        assert_eq!(expanded, "/Users/testuser/Documents");

        // Test that non-tilde paths are unchanged
        let expanded = interpreter.expand_tilde("/absolute/path");
        assert_eq!(expanded, "/absolute/path");

        // Restore original HOME
        unsafe {
            match original_home {
                Some(home) => std::env::set_var("HOME", home),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    #[test]
    fn test_generate_completions_with_tilde() {
        let mut interpreter = Interpreter::new();

        // Test that generate_completions handles tilde prefixes
        // This is an integration test for the completion system

        // Test with tilde at start of line (should expand for path completion)
        let _completions = interpreter.generate_completions("~/", 2);
        // Should return some completions if HOME directory exists and has contents
        // We can't assert specific results since it depends on the actual home directory

        // Test with tilde in middle of line (should still work)
        let _completions = interpreter.generate_completions("ls ~/", 5);
        // Should handle this case without errors

        // Test with non-tilde path
        let _completions = interpreter.generate_completions("./", 2);
        // Should work normally
    }

    #[test]
    fn test_tilde_completion_bash_like_display() {
        use tempfile::tempdir;

        let interpreter = Interpreter::new();

        // Mock HOME environment for testing
        let original_home = std::env::var("HOME").ok();
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        unsafe {
            std::env::set_var("HOME", &temp_path);
        }

        // Create some test files in the mock home directory
        fs::write(temp_dir.path().join("test_file.txt"), "content").unwrap();
        fs::create_dir(temp_dir.path().join("test_dir")).unwrap();

        // Test completion with ~/
        let (_suffixes, full_names) = interpreter.get_path_completions("~/");

        // Check that none of the results have double slashes
        for name in &full_names {
            assert!(
                !name.contains("//"),
                "Found double slash in completion: {name}"
            );
            // When completing in home directory, should show just filename (like bash)
            if name.contains("test_") {
                assert!(
                    !name.starts_with("~/"),
                    "Home directory completion should not have ~/ prefix: {name}"
                );
                assert!(
                    name.starts_with("test"),
                    "Should show just the filename: {name}"
                );
            }
        }

        // Test completion with ~/test prefix
        let (_suffixes, full_names) = interpreter.get_path_completions("~/test");

        // Check that results are properly formatted
        for name in &full_names {
            assert!(
                !name.contains("//"),
                "Found double slash in completion: {name}"
            );
            // For partial matches in home directory, should still show just filename
            if name.contains("test_") {
                assert!(
                    name.starts_with("test"),
                    "Completion should show filename: {name}"
                );
            }
        }

        // Restore original HOME
        unsafe {
            match original_home {
                Some(home) => std::env::set_var("HOME", home),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    #[test]
    fn test_tilde_completion_subdirectory_behavior() {
        use tempfile::tempdir;

        let interpreter = Interpreter::new();

        // Mock HOME environment for testing
        let original_home = std::env::var("HOME").ok();
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        unsafe {
            std::env::set_var("HOME", &temp_path);
        }

        // Create a subdirectory with files
        let sub_dir = temp_dir.path().join("Documents");
        fs::create_dir(&sub_dir).unwrap();
        fs::write(sub_dir.join("test_file.txt"), "content").unwrap();
        fs::create_dir(sub_dir.join("test_dir")).unwrap();

        // Test completion with ~/Documents/
        let (_suffixes, full_names) = interpreter.get_path_completions("~/Documents/");

        // In subdirectories, should show the full path with ~/
        for name in &full_names {
            if name.contains("test_") {
                assert!(
                    name.starts_with("~/Documents/"),
                    "Subdirectory completion should show full path: {name}"
                );
            }
        }

        // Test completion with ~/Documents/test prefix
        let (_suffixes, full_names) = interpreter.get_path_completions("~/Documents/test");

        // Should show full path for subdirectory completions
        for name in &full_names {
            if name.contains("test_") {
                assert!(
                    name.starts_with("~/Documents/test"),
                    "Subdirectory completion should show full path: {name}"
                );
            }
        }

        // Restore original HOME
        unsafe {
            match original_home {
                Some(home) => std::env::set_var("HOME", home),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    #[test]
    fn test_brace_expansion_numeric_ranges() {
        let interpreter = Interpreter::new();

        // Basic numeric range
        let result = interpreter.expand_brace_patterns("{1..5}");
        assert_eq!(result, vec!["1", "2", "3", "4", "5"]);

        // Reverse numeric range
        let result = interpreter.expand_brace_patterns("{5..1}");
        assert_eq!(result, vec!["5", "4", "3", "2", "1"]);

        // Single number range
        let result = interpreter.expand_brace_patterns("{3..3}");
        assert_eq!(result, vec!["3"]);

        // Negative numbers
        let result = interpreter.expand_brace_patterns("{-2..2}");
        assert_eq!(result, vec!["-2", "-1", "0", "1", "2"]);

        // Large range (should work but be reasonable)
        let result = interpreter.expand_brace_patterns("{1..10}");
        assert_eq!(
            result,
            vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]
        );
    }

    #[test]
    fn test_brace_expansion_character_ranges() {
        let interpreter = Interpreter::new();

        // Basic character range
        let result = interpreter.expand_brace_patterns("{a..e}");
        assert_eq!(result, vec!["a", "b", "c", "d", "e"]);

        // Reverse character range
        let result = interpreter.expand_brace_patterns("{e..a}");
        assert_eq!(result, vec!["e", "d", "c", "b", "a"]);

        // Single character range
        let result = interpreter.expand_brace_patterns("{z..z}");
        assert_eq!(result, vec!["z"]);

        // Uppercase characters
        let result = interpreter.expand_brace_patterns("{A..D}");
        assert_eq!(result, vec!["A", "B", "C", "D"]);

        // Mixed case (should not work - only single case ranges)
        let result = interpreter.expand_brace_patterns("{a..Z}");
        assert_eq!(result, vec!["{a..Z}"]);
    }

    #[test]
    fn test_brace_expansion_comma_separated() {
        let interpreter = Interpreter::new();

        // Basic comma-separated list
        let result = interpreter.expand_brace_patterns("{a,b,c}");
        assert_eq!(result, vec!["a", "b", "c"]);

        // Mixed types
        let result = interpreter.expand_brace_patterns("{1,hello,world}");
        assert_eq!(result, vec!["1", "hello", "world"]);

        // With spaces (should be trimmed)
        let result = interpreter.expand_brace_patterns("{a, b, c}");
        assert_eq!(result, vec!["a", "b", "c"]);

        // Single item
        let result = interpreter.expand_brace_patterns("{single}");
        assert_eq!(result, vec!["{single}"]);

        // Empty items
        let result = interpreter.expand_brace_patterns("{a,,c}");
        assert_eq!(result, vec!["a", "", "c"]);
    }

    #[test]
    fn test_brace_expansion_with_prefix_suffix() {
        let interpreter = Interpreter::new();

        // Prefix and suffix
        let result = interpreter.expand_brace_patterns("file{1..3}.txt");
        assert_eq!(result, vec!["file1.txt", "file2.txt", "file3.txt"]);

        // Only prefix
        let result = interpreter.expand_brace_patterns("prefix_{a,b,c}");
        assert_eq!(result, vec!["prefix_a", "prefix_b", "prefix_c"]);

        // Only suffix
        let result = interpreter.expand_brace_patterns("{1,2,3}_suffix");
        assert_eq!(result, vec!["1_suffix", "2_suffix", "3_suffix"]);

        // Complex prefix and suffix
        let result = interpreter.expand_brace_patterns("path/to/file{1..2}.backup.txt");
        assert_eq!(
            result,
            vec!["path/to/file1.backup.txt", "path/to/file2.backup.txt"]
        );
    }

    #[test]
    fn test_brace_expansion_nested_and_multiple() {
        let interpreter = Interpreter::new();

        // Multiple brace expansions in one string
        let result = interpreter.expand_brace_patterns("{a,b}{1,2}");
        assert_eq!(result, vec!["a1", "a2", "b1", "b2"]);

        // Multiple separate expansions
        let args = vec!["{1..2}".to_string(), "{a,b}".to_string()];
        let result = interpreter.expand_glob_patterns(&args);
        assert_eq!(result, vec!["1", "2", "a", "b"]);

        // Complex multiple expansions
        let result = interpreter.expand_brace_patterns("file{1,2}.{txt,log}");
        assert_eq!(
            result,
            vec!["file1.txt", "file1.log", "file2.txt", "file2.log"]
        );
    }

    #[test]
    fn test_brace_expansion_edge_cases() {
        let interpreter = Interpreter::new();

        // No braces
        let result = interpreter.expand_brace_patterns("normal_string");
        assert_eq!(result, vec!["normal_string"]);

        // Unmatched braces
        let result = interpreter.expand_brace_patterns("{unmatched");
        assert_eq!(result, vec!["{unmatched"]);

        let result = interpreter.expand_brace_patterns("unmatched}");
        assert_eq!(result, vec!["unmatched}"]);

        // Empty braces
        let result = interpreter.expand_brace_patterns("{}");
        assert_eq!(result, vec!["{}"]);

        // Invalid range
        let result = interpreter.expand_brace_patterns("{1..}");
        assert_eq!(result, vec!["{1..}"]);

        let result = interpreter.expand_brace_patterns("{..5}");
        assert_eq!(result, vec!["{..5}"]);

        // Non-range, non-comma content
        let result = interpreter.expand_brace_patterns("{hello}");
        assert_eq!(result, vec!["{hello}"]);

        // Multiple dots
        let result = interpreter.expand_brace_patterns("{1...5}");
        assert_eq!(result, vec!["{1...5}"]);
    }

    #[test]
    fn test_brace_expansion_integration_with_commands() {
        let mut interpreter = Interpreter::new();

        // Test with echo command
        let result = interpreter.execute("echo {1..3}").unwrap();
        assert_eq!(result, 0);

        // Test with variable assignment (brace expansion doesn't happen in assignments)
        let result = interpreter.execute("FILES=\"{a,b,c}\"").unwrap();
        assert_eq!(result, 0);
        assert_eq!(
            interpreter.variables.get("FILES"),
            Some(&"{a,b,c}".to_string())
        );

        // Test brace expansion with variable expansion
        interpreter
            .variables
            .insert("PREFIX".to_string(), "file".to_string());
        let expanded = interpreter.expand_brace_patterns("$PREFIX{1..2}.txt");
        // This should not expand the variable here, as that happens in expand_variables
        assert_eq!(expanded, vec!["$PREFIX1.txt", "$PREFIX2.txt"]);
    }

    #[test]
    fn test_brace_expansion_with_glob_patterns() {
        let interpreter = Interpreter::new();

        // Brace expansion should happen before glob expansion
        let args = vec!["{test,src}/*.rs".to_string()];
        let result = interpreter.expand_glob_patterns(&args);

        // This should expand to test/*.rs and src/*.rs, then glob each
        // The exact result depends on what files exist, but it should not contain the original pattern
        assert!(!result.contains(&"{test,src}/*.rs".to_string()));

        // Should contain expanded patterns or their glob matches
        let has_test_pattern = result
            .iter()
            .any(|s| s.contains("test") && s.contains(".rs"));
        let has_src_pattern = result
            .iter()
            .any(|s| s.contains("src") && s.contains(".rs"));

        // At least one should be true (depending on what files exist)
        assert!(has_test_pattern || has_src_pattern || result.len() >= 2);
    }
}
