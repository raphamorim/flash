pub mod lexer;
pub mod parser;

use crate::lexer::Lexer;
use crate::lexer::TokenKind;
use crate::parser::Node;
use crate::parser::Parser;
use crate::parser::RedirectKind;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::process::{Command, Stdio};

/// Main entry point for the shell parser/interpreter
fn main() -> io::Result<()> {
    let mut interpreter = Interpreter::new();
    interpreter.run_interactive()?;
    Ok(())
}

/// Shell interpreter
struct Interpreter {
    variables: HashMap<String, String>,
    last_exit_code: i32,
}

impl Interpreter {
    fn new() -> Self {
        let mut variables = HashMap::new();

        // Initialize some basic environment variables
        for (key, value) in env::vars() {
            variables.insert(key, value);
        }

        // Set up some shell variables
        variables.insert("?".to_string(), "0".to_string());
        variables.insert("SHELL".to_string(), "bash".to_string());

        Self {
            variables,
            last_exit_code: 0,
        }
    }

    fn run_interactive(&mut self) -> io::Result<()> {
        let mut buffer = String::new();
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            stdout.write_all(b"$ ")?;
            stdout.flush()?;

            buffer.clear();
            stdin.read_line(&mut buffer)?;

            if buffer.trim() == "exit" {
                break;
            }

            let result = self.execute(&buffer);

            match result {
                Ok(code) => {
                    self.last_exit_code = code;
                    self.variables.insert("?".to_string(), code.to_string());
                }
                Err(e) => {
                    println!("Error: {}", e);
                    self.last_exit_code = 1;
                    self.variables.insert("?".to_string(), "1".to_string());
                }
            }
        }

        Ok(())
    }

    fn execute(&mut self, input: &str) -> Result<i32, io::Error> {
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let ast = parser.parse_script();

        self.evaluate(&ast)
    }

    fn evaluate(&mut self, node: &Node) -> Result<i32, io::Error> {
        match node {
            Node::Command {
                name,
                args,
                redirects,
            } => {
                // Handle built-in commands
                match name.as_str() {
                    "cd" => {
                        let dir = if args.is_empty() {
                            env::var("HOME").unwrap_or_else(|_| ".".to_string())
                        } else {
                            args[0].clone()
                        };

                        match env::set_current_dir(&dir) {
                            Ok(_) => {
                                self.variables.insert(
                                    "PWD".to_string(),
                                    env::current_dir()?.to_string_lossy().to_string(),
                                );
                                Ok(0)
                            }
                            Err(e) => {
                                eprintln!("cd: {}: {}", dir, e);
                                Ok(1)
                            }
                        }
                    }
                    "echo" => {
                        // Simple echo implementation
                        for (i, arg) in args.iter().enumerate() {
                            print!("{}{}", if i > 0 { " " } else { "" }, arg);
                        }
                        println!();
                        Ok(0)
                    }
                    "export" => {
                        for arg in args {
                            if let Some(pos) = arg.find('=') {
                                let (key, value) = arg.split_at(pos);
                                let value = &value[1..]; // Skip the '='
                                self.variables.insert(key.to_string(), value.to_string());
                                unsafe {
                                    env::set_var(key, value);
                                }
                            } else {
                                // Just export an existing variable to the environment
                                if let Some(value) = self.variables.get(arg) {
                                    unsafe {
                                        env::set_var(arg, value);
                                    }
                                }
                            }
                        }
                        Ok(0)
                    }
                    "source" | "." => {
                        if args.is_empty() {
                            eprintln!("source: filename argument required");
                            return Ok(1);
                        }

                        let filename = &args[0];
                        match fs::read_to_string(filename) {
                            Ok(content) => self.execute(&content),
                            Err(e) => {
                                eprintln!("source: {}: {}", filename, e);
                                Ok(1)
                            }
                        }
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
                                        .write(true)
                                        .create(true)
                                        .append(true)
                                        .open(&redirect.file)?;
                                    command.stdout(Stdio::from(file));
                                }
                            }
                        }

                        // Set environment variables
                        for (key, value) in &self.variables {
                            command.env(key, value);
                        }

                        match command.status() {
                            Ok(status) => Ok(status.code().unwrap_or(0)),
                            Err(e) => {
                                eprintln!("{}: command not found", name);
                                Ok(127) // Command not found
                            }
                        }
                    }
                }
            }
            Node::Pipeline { commands } => {
                // Handle pipeline with proper piping
                if commands.is_empty() {
                    return Ok(0);
                }

                if commands.len() == 1 {
                    return self.evaluate(&commands[0]);
                }

                // For multiple commands, set up pipes
                let mut previous_output: Option<std::process::ChildStdout> = None;
                let commands_count = commands.len();

                for (i, command) in commands.iter().enumerate() {
                    match command {
                        Node::Command {
                            name,
                            args,
                            redirects,
                        } => {
                            let mut cmd = Command::new(name);
                            cmd.args(args);

                            // Set up stdin from previous command's stdout
                            if let Some(output) = previous_output.take() {
                                cmd.stdin(Stdio::from(output));
                            }

                            // Set up stdout pipe for all but the last command
                            if i < commands_count - 1 {
                                cmd.stdout(Stdio::piped());
                            }

                            // Apply redirects
                            for redirect in redirects {
                                match redirect.kind {
                                    RedirectKind::Input => {
                                        if i == 0 {
                                            // Only apply input redirect to first command
                                            let file = fs::File::open(&redirect.file)?;
                                            cmd.stdin(Stdio::from(file));
                                        }
                                    }
                                    RedirectKind::Output => {
                                        if i == commands_count - 1 {
                                            // Only apply output redirect to last command
                                            let file = fs::File::create(&redirect.file)?;
                                            cmd.stdout(Stdio::from(file));
                                        }
                                    }
                                    RedirectKind::Append => {
                                        if i == commands_count - 1 {
                                            // Only apply append redirect to last command
                                            let file = fs::OpenOptions::new()
                                                .write(true)
                                                .create(true)
                                                .append(true)
                                                .open(&redirect.file)?;
                                            cmd.stdout(Stdio::from(file));
                                        }
                                    }
                                }
                            }

                            // Set environment variables
                            for (key, value) in &self.variables {
                                cmd.env(key, value);
                            }

                            // Execute the command
                            let mut child = cmd.spawn()?;

                            // For all but the last command, capture the stdout
                            if i < commands_count - 1 {
                                previous_output = child.stdout.take();
                            } else {
                                // Wait for the last command to finish
                                let status = child.wait()?;
                                return Ok(status.code().unwrap_or(0));
                            }
                        }
                        _ => {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "Expected a Command in the pipeline",
                            ));
                        }
                    }
                }

                Ok(0)
            }
            Node::List {
                statements,
                operators,
            } => {
                let mut last_exit_code = 0;

                for (i, statement) in statements.iter().enumerate() {
                    last_exit_code = self.evaluate(statement)?;

                    // Check operators for control flow
                    if i < operators.len() {
                        match operators[i].as_str() {
                            "&&" => {
                                if last_exit_code != 0 {
                                    // Short-circuit on failure
                                    break;
                                }
                            }
                            "||" => {
                                if last_exit_code == 0 {
                                    // Short-circuit on success
                                    break;
                                }
                            }
                            _ => {} // Continue normally for ";" and "\n"
                        }
                    }
                }

                Ok(last_exit_code)
            }
            Node::Assignment { name, value } => {
                // Handle different types of values for assignment
                match &**value {
                    Node::StringLiteral(string_value) => {
                        // Expand variables in the value
                        let expanded_value = self.expand_variables(string_value);
                        self.variables.insert(name.clone(), expanded_value);
                    }
                    Node::CommandSubstitution { command } => {
                        // Execute command and capture output
                        let output = self.capture_command_output(command)?;
                        self.variables.insert(name.clone(), output);
                    }
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Unsupported value type for assignment",
                        ));
                    }
                }
                Ok(0)
            }
            Node::CommandSubstitution { command: _ } => {
                // This should be handled by the caller
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Unexpected command substitution node",
                ))
            }
            Node::StringLiteral(_value) => {
                // Just a placeholder, should be handled by parent node
                Ok(0)
            }
            Node::Subshell { list } => {
                // Not implemented: proper subshell environment
                // This just evaluates the list in the current environment
                self.evaluate(list)
            }
            Node::Comment(_) => Ok(0),
            &parser::Node::VariableAssignmentCommand { .. } => todo!(),
        }
    }

    // Method to capture command output for command substitution
    fn capture_command_output(&mut self, node: &Node) -> Result<String, io::Error> {
        // Create a temporary interpreter for the subshell
        let mut temp_interpreter = Interpreter::new();

        // Copy all variables to the temporary interpreter
        for (key, value) in &self.variables {
            temp_interpreter
                .variables
                .insert(key.clone(), value.clone());
        }

        // Set up pipes to capture stdout
        let (mut reader, writer) = os_pipe::pipe()?;
        let writer_clone = writer.try_clone()?;

        // Temporarily replace stdout
        let _old_stdout = std::io::stdout();
        let _handle = unsafe {
            let writer_raw_fd = writer.as_raw_fd();
            libc::dup2(writer_raw_fd, libc::STDOUT_FILENO)
        };

        // Execute the command
        let exit_code = temp_interpreter.evaluate(node)?;

        // Close the write end to avoid deadlock
        drop(writer);
        drop(writer_clone);

        // Read the output
        let mut output = String::new();
        reader.read_to_string(&mut output)?;

        // Trim the trailing newline if present
        if output.ends_with('\n') {
            output.pop();
        }

        // Check exit code
        if exit_code != 0 {
            self.last_exit_code = exit_code;
            self.variables
                .insert("?".to_string(), exit_code.to_string());
        }

        Ok(output)
    }

    fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' && chars.peek().is_some() {
                let mut var_name = String::new();

                // Variable can be specified as ${VAR} or $VAR
                if let Some(&'{') = chars.peek() {
                    chars.next(); // Skip '{'

                    // Read until closing brace
                    while let Some(c) = chars.next() {
                        if c == '}' {
                            break;
                        }
                        var_name.push(c);
                    }
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

                // Replace with variable value if exists
                if let Some(value) = self.variables.get(&var_name) {
                    result.push_str(value);
                }
            } else {
                result.push(c);
            }
        }

        result
    }
}

/// Formatter for shell scripts
struct Formatter {
    indent_level: usize,
    indent_str: String,
}

impl Formatter {
    fn new(indent_str: &str) -> Self {
        Self {
            indent_level: 0,
            indent_str: indent_str.to_string(),
        }
    }

    #[inline]
    fn set_indent_level(&mut self, level: usize) {
        self.indent_level = level;
    }

    fn format(&mut self, node: &Node) -> String {
        match node {
            Node::Command {
                name,
                args,
                redirects,
            } => {
                let mut result = self.indent();
                result.push_str(name);

                for arg in args {
                    result.push(' ');
                    // Quote arguments with spaces
                    if arg.contains(' ') {
                        result.push('"');
                        result.push_str(arg);
                        result.push('"');
                    } else {
                        result.push_str(arg);
                    }
                }

                for redirect in redirects {
                    result.push(' ');
                    result.push_str(&match redirect.kind {
                        RedirectKind::Input => "<",
                        RedirectKind::Output => ">",
                        RedirectKind::Append => ">>",
                    });
                    result.push(' ');
                    result.push_str(&redirect.file);
                }

                result
            }
            Node::Pipeline { commands } => {
                let mut parts = Vec::new();
                for cmd in commands {
                    parts.push(self.format(cmd));
                }
                parts.join(" | ")
            }
            Node::List {
                statements,
                operators,
            } => {
                let mut result = String::new();

                for (i, statement) in statements.iter().enumerate() {
                    if i > 0 {
                        result.push_str(&operators[i - 1]);
                        if operators[i - 1] == "\n" {
                            result.push('\n');
                        } else {
                            result.push(' ');
                        }
                    }

                    result.push_str(&self.format(statement));
                }

                result
            }
            Node::Assignment { name, value } => {
                let mut result = self.indent();
                result.push_str(name);
                result.push('=');

                match &**value {
                    Node::StringLiteral(val) => {
                        // Quote value if it contains spaces
                        if val.contains(' ') {
                            result.push('"');
                            result.push_str(val);
                            result.push('"');
                        } else {
                            result.push_str(val);
                        }
                    }
                    Node::CommandSubstitution { command } => {
                        result.push_str("$(");
                        result.push_str(&self.format(command));
                        result.push(')');
                    }
                    _ => {
                        result.push_str("<unknown>");
                    }
                }

                result
            }
            Node::CommandSubstitution { command } => {
                let mut result = String::new();
                result.push_str("$(");
                result.push_str(&self.format(command));
                result.push(')');
                result
            }
            Node::StringLiteral(value) => {
                let mut result = String::new();
                // Quote if contains spaces
                if value.contains(' ') {
                    result.push('"');
                    result.push_str(value);
                    result.push('"');
                } else {
                    result.push_str(value);
                }
                result
            }
            Node::Subshell { list } => {
                let mut result = self.indent();
                result.push_str("( ");

                self.indent_level += 1;
                result.push_str(&self.format(list));
                self.indent_level -= 1;

                result.push_str(" )");
                result
            }
            Node::Comment(comment) => {
                let mut result = self.indent();
                result.push_str(comment);
                result
            }
            &parser::Node::VariableAssignmentCommand { .. } => todo!(),
        }
    }

    fn indent(&self) -> String {
        self.indent_str.repeat(self.indent_level)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_lexer() {
//         let input = "echo hello | grep world";
//         let mut lexer = Lexer::new(input);

//         let expected_tokens = vec![
//             TokenKind::Word("echo".to_string()),
//             TokenKind::Word("hello".to_string()),
//             TokenKind::Pipe,
//             TokenKind::Word("grep".to_string()),
//             TokenKind::Word("world".to_string()),
//             TokenKind::EOF,
//         ];

//         for expected in expected_tokens {
//             let token = lexer.next_token();
//             assert_eq!(token.kind, expected);
//         }
//     }

//     #[test]
//     fn test_parser() {
//         let input = "echo hello > output.txt";
//         let lexer = Lexer::new(input);
//         let mut parser = Parser::new(lexer);

//         if let Node::Command {
//             name,
//             args,
//             redirects,
//         } = parser.parse_command()
//         {
//             assert_eq!(name, "echo");
//             assert_eq!(args, vec!["hello"]);
//             assert_eq!(redirects.len(), 1);
//             assert!(matches!(redirects[0].kind, RedirectKind::Output));
//             assert_eq!(redirects[0].file, "output.txt");
//         } else {
//             panic!("Expected Command node");
//         }
//     }

//     #[test]
//     fn test_formatter() {
//         let input = "if [ -f /etc/bashrc ]; then\nsource /etc/bashrc\nfi";
//         let formatted = format_script_with_options(input, " ", 0);

//         // This is a simplified test. In a real formatter, this would actually parse the if/then/fi
//         // constructs correctly
//         assert!(formatted.contains("source /etc/bashrc"));
//     }

//     #[test]
//     fn test_variable_expansion() {
//         let mut interpreter = Interpreter::new();
//         interpreter
//             .variables
//             .insert("NAME".to_string(), "world".to_string());

//         let expanded = interpreter.expand_variables("Hello $NAME!");
//         assert_eq!(expanded, "Hello world!");

//         let expanded = interpreter.expand_variables("Hello ${NAME}!");
//         assert_eq!(expanded, "Hello world!");
//     }

//     #[test]
//     fn test_command_execution() {
//         let mut interpreter = Interpreter::new();

//         // Test a basic command
//         let result = interpreter.execute("echo test").unwrap();
//         assert_eq!(result, 0);

//         // Test assignment
//         let result = interpreter.execute("X=test").unwrap();
//         assert_eq!(result, 0);
//         assert_eq!(interpreter.variables.get("X"), Some(&"test".to_string()));
//     }

//     // #[test]
//     // fn test_pipeline() {
//     //     let input = "echo hello | grep e";
//     //     let lexer = Lexer::new(input);
//     //     let mut parser = Parser::new(lexer);

//     //     if let Node::Pipeline { commands } = parser.parse_command() {
//     //         assert_eq!(commands.len(), 2);

//     //         if let Node::Command { name, args, .. } = &commands[0] {
//     //             assert_eq!(name, "echo");
//     //             assert_eq!(args, &["hello"]);
//     //         } else {
//     //             panic!("Expected Command node");
//     //         }

//     //         if let Node::Command { name, args, .. } = &commands[1] {
//     //             assert_eq!(name, "grep");
//     //             assert_eq!(args, &["e"]);
//     //         } else {
//     //             panic!("Expected Command node");
//     //         }
//     //     } else {
//     //         panic!("Expected Pipeline node");
//     //     }
//     // }

//     #[test]
//     fn test_complex_pipeline() {
//         let input = "cat file.txt | grep pattern | sort | uniq -c | sort -nr";
//         let lexer = Lexer::new(input);
//         let mut parser = Parser::new(lexer);

//         if let Node::Pipeline { commands } = parser.parse_command() {
//             assert_eq!(commands.len(), 5);

//             if let Node::Command { name, .. } = &commands[0] {
//                 assert_eq!(name, "cat");
//             }

//             if let Node::Command { name, .. } = &commands[4] {
//                 assert_eq!(name, "sort");
//             }
//         } else {
//             panic!("Expected Pipeline node");
//         }
//     }

//     #[test]
//     fn test_subshell() {
//         let input = "(cd /tmp && ls)";
//         let lexer = Lexer::new(input);
//         let mut parser = Parser::new(lexer);

//         if let Node::Subshell { list } = parser.parse_statement().unwrap() {
//             if let Node::List {
//                 statements,
//                 operators,
//             } = list.as_ref()
//             {
//                 assert_eq!(statements.len(), 2);
//                 assert_eq!(operators, &["&&".to_string()]);
//             } else {
//                 panic!("Expected List node");
//             }
//         } else {
//             panic!("Expected Subshell node");
//         }
//     }

//     #[test]
//     fn test_logical_operators() {
//         let input = "true && echo success || echo failure";
//         let lexer = Lexer::new(input);
//         let mut parser = Parser::new(lexer);
//         let node = parser.parse_script();

//         if let Node::List {
//             statements,
//             operators,
//         } = node
//         {
//             assert_eq!(statements.len(), 3);
//             assert_eq!(operators, &["&&".to_string(), "||".to_string()]);
//         } else {
//             panic!("Expected List node");
//         }
//     }

//     #[test]
//     fn test_comments() {
//         let input = "echo hello # this is a comment\necho world";
//         let lexer = Lexer::new(input);
//         let mut parser = Parser::new(lexer);
//         let node = parser.parse_script();

//         if let Node::List { statements, .. } = node {
//             assert_eq!(statements.len(), 3); // echo hello, comment, echo world

//             match &statements[1] {
//                 Node::Comment(text) => {
//                     assert!(text.starts_with("# this is a comment"));
//                 }
//                 _ => panic!("Expected Comment node"),
//             }
//         } else {
//             panic!("Expected List node");
//         }
//     }

// //     #[test]
// //     fn integration_test_basic_script_with_variable_and_if_and_else() {
// //         let script = r#"
// //     #!/bin/bash
// //     # This is a test script
// //     echo "Starting test"
// //     RESULT=$(echo "test" | grep "t")
// //     echo "Result: $RESULT"
// //     if [ -f "/tmp/test" ]; then
// //         echo "File exists"
// //     else
// //         echo "File doesn't exist"
// //     fi
// //     "#;

// //         let mut interpreter = Interpreter::new();
// //         let result = interpreter.execute(script).unwrap();

// //         // Just make sure it runs without errors
// //         assert_eq!(result, 0);
// //     }

// //     #[test]
// //     fn integration_test_basic_script() {
// //         let script = r#"
// // # Simple test script with basic commands only
// // echo "Starting test"
// // MESSAGE="Hello world"
// // echo "Message: $MESSAGE"
// // cd /tmp
// // echo "Current directory: $(pwd)"
// // "#;

// //         let mut interpreter = Interpreter::new();
// //         let result = interpreter.execute(script).unwrap();

// //         // Just make sure it runs without errors
// //         assert_eq!(result, 0);
// //     }

//     #[test]
//     fn integration_test_formatter() {
//         let script = "if [ $x -eq 42 ]; then echo \"The answer\"; fi";
//         let formatted = format_script_with_options(script, "  ", 0);

//         println!("{:?}", formatted);

//         // Check that the formatter adds appropriate whitespace
//         assert!(formatted.contains("if [ $x -eq 42 ]"));
//         assert!(formatted.contains("echo \"The answer\""));
//     }
// }

// // Example usage of the library components

// fn example_usage() {
//     // Example 1: Parse and execute a simple script
//     let script = "echo Hello, world!";
//     let mut interpreter = Interpreter::new();
//     let exit_code = interpreter.execute(script).unwrap();
//     println!("Script executed with exit code: {}", exit_code);

//     // Example 2: Format a script
//     let script = "if [ $x -eq 42 ]; then echo \"The answer\"; fi";
//     let formatted = format_script_with_options(script, "  ", 0);
//     println!("Formatted script:\n{}", formatted);

//     // Example 3: Lexer and parser usage
//     let script = "echo $HOME | grep '/home'";
//     let lexer = Lexer::new(script);
//     let mut parser = Parser::new(lexer);
//     let ast = parser.parse_script();

//     // We could implement a proper Debug implementation to print the AST
//     println!("AST: {:?}", ast);
// }

// // Utility functions

// /// Parse a shell script and return its AST
// fn parse_script(script: &str) -> Node {
//     let lexer = Lexer::new(script);
//     let mut parser = Parser::new(lexer);
//     parser.parse_script()
// }

// /// Execute a shell script and return the exit code
// fn execute_script(script: &str) -> Result<i32, io::Error> {
//     let mut interpreter = Interpreter::new();
//     interpreter.execute(script)
// }

// /// Format a shell script with the specified indentation
// fn format_script_with_options(script: &str, indent: &str, indent_level: usize) -> String {
//     let ast = parse_script(script);
//     let mut formatter = Formatter::new(indent);
//     formatter.set_indent_level(indent_level);
//     formatter.format(&ast)
// }
