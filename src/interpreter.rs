/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use crate::flash;
use crate::lexer::Lexer;
use crate::parser::Node;
use crate::parser::Parser;
use crate::parser::Redirect;
use crate::parser::RedirectKind;

use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use termios::{ECHO, ICANON, TCSANOW, Termios, VMIN, VTIME, tcsetattr};

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
            Node::CommandSubstitution { command: _ } => {
                Err(io::Error::other("Unexpected command substitution node"))
            }
            Node::StringLiteral(_value) => Ok(0),
            Node::Subshell { list } => interpreter.evaluate_with_evaluator(list, self),
            Node::Comment(_) => Ok(0),
            Node::ExtGlobPattern {
                operator,
                patterns,
                suffix,
            } => self.evaluate_ext_glob(*operator, patterns, suffix, interpreter),
            Node::Export { name, value } => self.evaluate_export(name, value, interpreter),
            _ => Err(io::Error::other("Unsupported node type")),
        }
    }
}

impl DefaultEvaluator {
    fn evaluate_command(
        &mut self,
        name: &str,
        args: &[String],
        redirects: &[Redirect],
        interpreter: &mut Interpreter,
    ) -> Result<i32, io::Error> {
        // Handle built-in commands
        match name {
            "cd" => {
                let dir = if args.is_empty() {
                    env::var("HOME").unwrap_or_else(|_| ".".to_string())
                } else {
                    args[0].clone()
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
                        eprintln!("cd: {}: {}", dir, e);
                        Ok(1)
                    }
                }
            }
            "echo" => {
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
                        let value = &value[1..];
                        interpreter
                            .variables
                            .insert(key.to_string(), value.to_string());
                        unsafe {
                            env::set_var(key, value);
                        }
                    } else if let Some(value) = interpreter.variables.get(arg) {
                        unsafe {
                            env::set_var(arg, value);
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
                    Ok(content) => interpreter.execute(&content),
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
                                .create(true)
                                .append(true)
                                .open(&redirect.file)?;
                            command.stdout(Stdio::from(file));
                        }
                    }
                }

                // Set environment variables
                for (key, value) in &interpreter.variables {
                    command.env(key, value);
                }

                match command.status() {
                    Ok(status) => Ok(status.code().unwrap_or(0)),
                    Err(_) => {
                        eprintln!("{}: command not found", name);
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
        println!("{:?} {:?}", name, value);
        match value {
            Some(val) => {
                // Export with assignment: export VAR=value
                match val.as_ref() {
                    Node::StringLiteral(string_value) => {
                        let expanded_value = interpreter.expand_variables(string_value);
                        interpreter
                            .variables
                            .insert(name.to_string(), expanded_value.clone());
                        unsafe {
                            env::set_var(name, &expanded_value);
                        }
                    }
                    Node::CommandSubstitution { command } => {
                        let output = interpreter.capture_command_output(command, self)?;
                        let trimmed_output = output.trim_end().to_string();
                        interpreter
                            .variables
                            .insert(name.to_string(), trimmed_output.clone());
                        unsafe {
                            env::set_var(name, &trimmed_output);
                        }
                    }
                    Node::Array { elements } => {
                        // Handle array export - join elements with spaces or use a specific format
                        let array_value = elements.join(" ");
                        let expanded_value = interpreter.expand_variables(&array_value);
                        interpreter
                            .variables
                            .insert(name.to_string(), expanded_value.clone());
                        unsafe {
                            env::set_var(name, &expanded_value);
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
                    unsafe {
                        env::set_var(name, existing_value);
                    }
                } else {
                    // If variable doesn't exist in interpreter, check if it exists in environment
                    if let Ok(env_value) = env::var(name) {
                        // Store it in interpreter variables for consistency
                        interpreter
                            .variables
                            .insert(name.to_string(), env_value.clone());
                        unsafe {
                            env::set_var(name, &env_value);
                        }
                    } else {
                        // Variable doesn't exist anywhere, export with empty value
                        interpreter
                            .variables
                            .insert(name.to_string(), String::new());
                        unsafe {
                            env::set_var(name, "");
                        }
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
            last_exit_code = interpreter.evaluate_with_evaluator(statement, self)?;

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
            Node::CommandSubstitution { command } => {
                let output = interpreter.capture_command_output(command, self)?;
                interpreter.variables.insert(name.to_string(), output);
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
            println!("{}", m);
        }

        Ok(0)
    }
}

/// Shell interpreter
pub struct Interpreter {
    pub variables: HashMap<String, String>,
    pub last_exit_code: i32,
    pub history: Vec<String>,
    pub history_file: Option<String>,
    pub rc_file: Option<String>,
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
        }

        // Set up some shell variables
        variables.insert("?".to_string(), "0".to_string());
        variables.insert("SHELL".to_string(), "flash".to_string());
        variables.insert("$$".to_string(), std::process::id().to_string());

        let home_dir = env::var("HOME").ok();

        let history_file = home_dir
            .as_ref()
            .map(|home| format!("{}/.flash_history", home));

        let rc_file = home_dir.as_ref().map(|home| format!("{}/.flashrc", home));

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
            last_exit_code: 0,
            history,
            history_file,
            rc_file,
        };

        // Load and execute flashrc file if it exists
        if let Err(e) = interpreter.load_rc_file() {
            eprintln!("Warning: Error loading flashrc: {}", e);
        }

        interpreter
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
                            eprintln!("Warning: Error executing flashrc: {}", e);
                        }
                    }
                    Err(e) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("Failed to read flashrc file {}: {}", rc_path, e),
                        ));
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
                writeln!(file, "{}", line)?;
            }
        }
        Ok(())
    }

    // Generate completion candidates for the current input
    fn generate_completions(&self, input: &str, cursor_pos: usize) -> (Vec<String>, Vec<String>) {
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
            let vars: Vec<String> = self.variables.keys().map(|k| format!("${}", k)).collect();
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
                    .map(|k| format!("${}", k))
                    .collect();
                return (suffixes, full_names);
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
        for cmd in &["cd", "echo", "export", "source", ".", "exit"] {
            if cmd.starts_with(prefix) {
                full_names.push(cmd.to_string());
                if let Some(stripped) = cmd.strip_prefix(prefix) {
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

        // Determine the directory to search and the filename prefix
        let (dir_path, file_prefix) = if prefix.contains('/') {
            let path = Path::new(prefix);
            let parent = path.parent().unwrap_or(Path::new(""));
            let file_name = path.file_name().map_or("", |f| f.to_str().unwrap_or(""));
            (parent.to_path_buf(), file_name.to_string())
        } else {
            (PathBuf::from("."), prefix.to_string())
        };

        // Read the directory entries
        if let Ok(entries) = fs::read_dir(dir_path.clone()) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(&file_prefix) {
                        // For display, show the full path
                        let full_path = if prefix.contains('/') {
                            format!("{}/{}", dir_path.display(), name)
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
            print!("{:<width$}", completion, width = max_width);
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

    // Interactive shell that accepts a custom evaluator
    pub fn run_interactive_with_evaluator<E: Evaluator>(
        &mut self,
        mut evaluator: E,
    ) -> io::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let fd = stdin.as_raw_fd();

        let original_termios = Termios::from_fd(fd)?;
        let mut raw_termios = original_termios;

        let _guard = scopeguard::guard((), |_| {
            let _ = tcsetattr(fd, TCSANOW, &original_termios);
        });

        let mut history_index = self.history.len();

        loop {
            write!(stdout, "$ ")?;
            stdout.flush()?;

            let input = self.read_line_with_completion(
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
                    println!("Error: {}", e);
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
        &self,
        original_termios: &Termios,
        raw_termios: &mut Termios,
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
            raw_termios.c_lflag &= !(ICANON | ECHO);
            raw_termios.c_cc[VMIN] = 1;
            raw_termios.c_cc[VTIME] = 0;
            tcsetattr(fd, TCSANOW, raw_termios)?;

            // Read a single byte
            let mut input_byte = [0u8; 1];
            stdin.read_exact(&mut input_byte)?;

            // Switch back to canonical mode for printing
            tcsetattr(fd, TCSANOW, original_termios)?;

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
                            write!(stdout, "\r$ {}", buffer)?;
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
                                    write!(stdout, "\r$ {}", buffer)?;
                                    stdout.flush()?;
                                } else {
                                    // No common prefix, show all completions (using full names for display)
                                    self.display_completions(&full_names)?;
                                    // Redraw the prompt and line
                                    write!(stdout, "$ {}", buffer)?;
                                    stdout.flush()?;
                                }
                            } else {
                                // No common prefix found, show all completions (using full names for display)
                                self.display_completions(&full_names)?;
                                // Redraw the prompt and line
                                write!(stdout, "$ {}", buffer)?;
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
                        write!(stdout, "\r$ {}", buffer)?;
                        write!(stdout, " ")?; // Clear deleted character
                        write!(stdout, "\r$ {}", buffer)?;
                        stdout.flush()?;
                    }
                }

                // Ctrl-A (move to beginning of line)
                1 => {
                    cursor_pos = 0;
                    write!(stdout, "\r$ ")?;
                    stdout.flush()?;
                }

                // Ctrl-E (move to end of line)
                5 => {
                    cursor_pos = buffer.len();
                    write!(stdout, "\r$ {}", buffer)?;
                    stdout.flush()?;
                }

                // Ctrl-B (move back one character) - same as left arrow
                2 => {
                    if cursor_pos > 0 {
                        cursor_pos -= 1;
                        write!(stdout, "\r$ {}", buffer)?;
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
                        write!(stdout, "\r$ {}", buffer)?;
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
                        write!(stdout, "\r$ {}", buffer)?;
                        write!(stdout, "                    ")?; // Clear any leftovers
                        write!(stdout, "\r$ {}", buffer)?;
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
                        write!(stdout, "\r$ {}", buffer)?;
                        write!(stdout, "                    ")?; // Clear any leftovers
                        write!(stdout, "\r$ {}", buffer)?;
                        stdout.flush()?;
                    }
                }

                // Ctrl-Y (yank/paste previously killed text)
                25 => {
                    if !kill_ring.is_empty() {
                        buffer.insert_str(cursor_pos, &kill_ring);
                        cursor_pos += kill_ring.len();

                        // Redraw
                        write!(stdout, "\r$ {}", buffer)?;
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
                            write!(stdout, "\r$ {}", buffer)?;
                            write!(stdout, "                    ")?; // Clear any leftovers
                            write!(stdout, "\r$ {}", buffer)?;
                            stdout.flush()?;
                        }
                    }
                }

                // Ctrl-L (clear screen)
                12 => {
                    // Clear the screen and redraw the prompt
                    write!(stdout, "\x1B[2J\x1B[H")?; // ANSI escape sequence to clear screen and move cursor to home
                    write!(stdout, "$ {}", buffer)?;
                    stdout.flush()?;
                }

                // Ctrl-P (previous history) - same as up arrow
                16 => {
                    if *history_index > 0 {
                        *history_index -= 1;
                        buffer = self.history[*history_index].clone();
                        cursor_pos = buffer.len();
                        write!(stdout, "\r$ {}", buffer)?;
                        write!(stdout, "                    ")?; // Clear any leftovers
                        write!(stdout, "\r$ {}", buffer)?;
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
                        write!(stdout, "\r$ {}", buffer)?;
                        write!(stdout, "                    ")?; // Clear any leftovers
                        write!(stdout, "\r$ {}", buffer)?;
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
                        write!(stdout, "\r$ {}", buffer)?;
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
                        write!(stdout, "\r$ {}", buffer)?;
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
                        write!(stdout, "\r$ {}", buffer)?;
                        write!(stdout, " ")?; // Clear deleted character
                        write!(stdout, "\r$ {}", buffer)?;
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
                        raw_termios.c_lflag &= !(ICANON | ECHO);
                        tcsetattr(fd, TCSANOW, raw_termios)?;
                        let mut search_byte = [0u8; 1];
                        stdin.read_exact(&mut search_byte)?;
                        tcsetattr(fd, TCSANOW, original_termios)?;

                        match search_byte[0] {
                            // Enter - accept the current match
                            b'\n' | b'\r' => {
                                if found {
                                    write!(stdout, "\r\n")?;
                                    buffer = self.history[search_index].clone();
                                    cursor_pos = buffer.len();
                                } else {
                                    write!(stdout, "\r$ {}", original_buffer)?;
                                    cursor_pos = original_cursor_pos;
                                }
                                stdout.flush()?;
                                break;
                            }

                            // Escape - cancel search
                            27 => {
                                write!(stdout, "\r$ {}", original_buffer)?;
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
                                            "\r(failed reverse-i-search)`{}': ",
                                            search_term
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
                                            "\r(failed reverse-i-search)`{}': ",
                                            search_term
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
                    println!("^C");
                    return Ok(String::new());
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
                                    write!(stdout, "\r$ {}", buffer)?;
                                    write!(stdout, "                    ")?; // Clear any leftovers
                                    write!(stdout, "\r$ {}", buffer)?;
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
                                    write!(stdout, "\r$ {}", buffer)?;
                                    write!(stdout, "                    ")?; // Clear any leftovers
                                    write!(stdout, "\r$ {}", buffer)?;
                                    stdout.flush()?;
                                }
                            }

                            // Left arrow
                            b'D' => {
                                if cursor_pos > 0 {
                                    cursor_pos -= 1;
                                    write!(stdout, "\r$ {}", buffer)?;
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
                                    write!(stdout, "\r$ {}", buffer)?;
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
                        write!(stdout, "\r$ {}", buffer)?;
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
                Regex::new(&format!("^{}$", regex_str))
                    .unwrap_or_else(|_| Regex::new("^$").unwrap())
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

    pub fn capture_command_output<E: Evaluator>(
        &mut self,
        node: &Node,
        evaluator: &mut E,
    ) -> Result<String, io::Error> {
        let mut temp_interpreter = Interpreter::new();

        for (key, value) in &self.variables {
            temp_interpreter
                .variables
                .insert(key.clone(), value.clone());
        }

        // Use os_pipe to capture output (implementation depends on your pipe setup)
        let (mut reader, writer) = os_pipe::pipe()?;
        let _writer_clone = writer.try_clone()?;

        // Execute the command with the evaluator
        let exit_code = temp_interpreter.evaluate_with_evaluator(node, evaluator)?;

        drop(writer);

        let mut output = String::new();
        reader.read_to_string(&mut output)?;

        if output.ends_with('\n') {
            output.pop();
        }

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
                    for c in chars.by_ref() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

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

        // Change to the temporary directory
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_path).unwrap();

        // Create interpreter
        let interpreter = Interpreter::new();

        // Test with prefix "test" - now returns (suffixes, full_names)
        let (suffixes, full_names) = interpreter.get_path_completions("test");
        assert!(
            full_names.contains(&"test1.txt".to_string())
                || full_names.contains(&"test2.txt".to_string())
                || full_names.contains(&"testdir/".to_string())
        );
        assert!(
            suffixes.contains(&"1.txt".to_string())
                || suffixes.contains(&"2.txt".to_string())
                || suffixes.contains(&"dir/".to_string())
        );

        // Test directory completion (should add trailing slash)
        let (suffixes, full_names) = interpreter.get_path_completions("testd");
        assert!(full_names.contains(&"testdir/".to_string()));
        assert!(suffixes.contains(&"ir/".to_string()));

        // Test with specific file prefix
        let (suffixes, full_names) = interpreter.get_path_completions("test1");
        assert!(full_names.contains(&"test1.txt".to_string()));
        assert!(suffixes.contains(&".txt".to_string()));
        assert!(!suffixes.contains(&"2.txt".to_string()));

        // Change back to original directory
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_generate_completions_for_commands() {
        let interpreter = Interpreter::new();

        // Test completion at beginning of line - now returns (suffixes, full_names)
        let (_suffixes, full_names) = interpreter.generate_completions("", 0);
        assert!(!full_names.is_empty());
        assert!(full_names.contains(&"cd".to_string()));

        // Test completion for partial command "ec"
        let (suffixes, full_names) = interpreter.generate_completions("ec", 2);
        assert!(full_names.contains(&"echo".to_string()));
        assert!(suffixes.contains(&"ho".to_string()));

        // Test completion after a space (should suggest commands)
        let (_suffixes, full_names) = interpreter.generate_completions("cd ", 3);
        assert!(!full_names.is_empty());
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

        // Change to the temporary directory
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_path).unwrap();

        // Create interpreter
        let interpreter = Interpreter::new();

        // Test completion with directory path
        let input = "cd dir1/";
        let (_suffixes, full_names) = interpreter.generate_completions(input, input.len());

        // Check if any completion contains "subdir" or "file.txt"
        let has_expected_completion = full_names
            .iter()
            .any(|c| c.contains("subdir") || c.contains("file.txt"));
        assert!(
            has_expected_completion,
            "Expected completions to contain subdir or file.txt, got: {:?}",
            full_names
        );

        // Test completion with partial path
        let input = "cd dir1/s";
        let (suffixes, _full_names) = interpreter.generate_completions(input, input.len());
        let has_subdir = suffixes.iter().any(|c| c.contains("ubdir"));
        assert!(
            has_subdir,
            "Expected suffixes to contain 'ubdir', got: {:?}",
            suffixes
        );

        // Test completion with file path
        let input = "cd dir1/f";
        let (suffixes, _full_names) = interpreter.generate_completions(input, input.len());
        let has_file = suffixes.iter().any(|c| c.contains("ile.txt"));
        assert!(
            has_file,
            "Expected suffixes to contain 'ile.txt', got: {:?}",
            suffixes
        );

        // Change back to original directory
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_completion_with_multiple_words() {
        let interpreter = Interpreter::new();

        // Test command completion after pipe
        let (suffixes, full_names) = interpreter.generate_completions("ls | e", 6);

        // Check that we get command completions starting with 'e'
        let has_echo_or_export = full_names.iter().any(|c| *c == "echo" || *c == "export");
        let has_echo_or_export_suffix = suffixes.iter().any(|c| *c == "cho" || *c == "xport");
        assert!(
            has_echo_or_export || has_echo_or_export_suffix,
            "Expected completions to include echo or export, got full_names: {:?}, suffixes: {:?}",
            full_names,
            suffixes
        );

        // Test path completion after command
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        fs::write(temp_path.join("testfile.txt"), "content").unwrap();

        let original_dir = env::current_dir().unwrap();
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
            "Expected completions to include 'testfile.txt', got full_names: {:?}, suffixes: {:?}",
            full_names,
            suffixes
        );

        env::set_current_dir(original_dir).unwrap();
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
            "Expected suffixes to include 'PATH', got: {:?}",
            suffixes
        );
        assert!(
            has_path_full,
            "Expected full_names to include '$TEST_PATH', got: {:?}",
            full_names
        );

        // Test completion right after $
        let (_suffixes, full_names) = interpreter.generate_completions("cd $", 4);
        let has_test_path = full_names.iter().any(|c| c == "$TEST_PATH");
        assert!(
            has_test_path,
            "Expected full_names to include '$TEST_PATH', got: {:?}",
            full_names
        );
    }
}
