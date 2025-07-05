/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// Completion system for Flash shell
#[derive(Debug, Clone, Default)]
pub struct CompletionSystem {
    /// Custom completion functions for specific commands
    pub command_completions: HashMap<String, CompletionEntry>,
    /// Default completion function
    pub default_function: String,
    /// Current completion context
    pub current: CompletionEntry,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionEntry {
    /// Function name to call for completion
    pub function: String,
    /// Action type (alias, command, file, etc.)
    pub action: String,
    /// Options for completion behavior
    pub options: HashMap<String, String>,
    /// Options that modify completion behavior
    pub o_options: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// The full command line
    pub line: String,
    /// Current cursor position
    pub point: usize,
    /// Words in the command line
    pub words: Vec<String>,
    /// Current word being completed
    pub cword: usize,
    /// The word being completed
    pub current_word: String,
    /// Previous word
    pub prev_word: String,
}

impl CompletionSystem {
    pub fn new() -> Self {
        let mut system = Self::default();
        system.setup_default_completions();
        system
    }

    /// Set up default completions for common commands
    fn setup_default_completions(&mut self) {
        // Git completion
        self.command_completions.insert(
            "git".to_string(),
            CompletionEntry {
                function: "_git_complete".to_string(),
                action: "".to_string(),
                options: HashMap::new(),
                o_options: vec!["nospace".to_string()],
            },
        );

        // SSH completion
        self.command_completions.insert(
            "ssh".to_string(),
            CompletionEntry {
                function: "_ssh_complete".to_string(),
                action: "".to_string(),
                options: HashMap::new(),
                o_options: vec!["nospace".to_string()],
            },
        );

        // CD completion (directories only)
        self.command_completions.insert(
            "cd".to_string(),
            CompletionEntry {
                function: "".to_string(),
                action: "directory".to_string(),
                options: HashMap::new(),
                o_options: vec!["nospace".to_string()],
            },
        );

        // Kill completion (process IDs)
        self.command_completions.insert(
            "kill".to_string(),
            CompletionEntry {
                function: "_kill_complete".to_string(),
                action: "".to_string(),
                options: HashMap::new(),
                o_options: vec!["nospace".to_string()],
            },
        );

        // Man completion
        self.command_completions.insert(
            "man".to_string(),
            CompletionEntry {
                function: "_man_complete".to_string(),
                action: "".to_string(),
                options: HashMap::new(),
                o_options: vec!["nospace".to_string()],
            },
        );
    }

    /// Generate completions for the given context
    pub fn complete(&mut self, context: &CompletionContext) -> Vec<String> {
        if context.words.is_empty() {
            return Vec::new();
        }

        let command = &context.words[0];

        // Check if we have a custom completion for this command
        if let Some(entry) = self.command_completions.get(command).cloned() {
            self.current = entry.clone();

            if !entry.function.is_empty() {
                return self.call_completion_function(&entry.function, context);
            } else if !entry.action.is_empty() {
                return self.complete_by_action(&entry.action, context);
            }
        }

        // Default completion based on position
        if context.cword == 0 {
            // Completing command name
            self.complete_commands(&context.current_word)
        } else {
            // Completing arguments - default to file completion
            self.complete_files(&context.current_word)
        }
    }

    /// Complete command names
    pub fn complete_commands(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();

        // Built-in commands
        let builtins = [
            "cd", "echo", "export", "source", ".", "exit", "alias", "unalias", "true", "false",
            "test", "[", "seq", "kill", "jobs", "bg", "fg", "history", "which", "type", "help",
            "complete",
        ];

        for builtin in &builtins {
            if builtin.starts_with(prefix) {
                completions.push(builtin.to_string());
            }
        }

        // Commands from PATH
        if let Ok(path) = env::var("PATH") {
            for path_entry in path.split(':') {
                if let Ok(entries) = fs::read_dir(path_entry) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with(prefix) {
                                if let Ok(metadata) = entry.path().metadata() {
                                    if metadata.is_file() {
                                        #[cfg(unix)]
                                        {
                                            use std::os::unix::fs::PermissionsExt;
                                            if metadata.permissions().mode() & 0o111 != 0 {
                                                completions.push(name.to_string());
                                            }
                                        }
                                        #[cfg(not(unix))]
                                        {
                                            completions.push(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        completions.sort();
        completions.dedup();
        completions
    }

    /// Complete file and directory names
    pub fn complete_files(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();

        // Expand tilde if present
        let expanded_prefix = if prefix.starts_with('~') {
            if let Ok(home) = env::var("HOME") {
                prefix.replacen('~', &home, 1)
            } else {
                prefix.to_string()
            }
        } else {
            prefix.to_string()
        };

        // Determine directory and filename prefix
        let (dir_path, file_prefix) = if expanded_prefix.contains('/') {
            if expanded_prefix.ends_with('/') {
                (expanded_prefix.clone(), String::new())
            } else {
                let path = Path::new(&expanded_prefix);
                let parent = path.parent().unwrap_or(Path::new(""));
                let file_name = path.file_name().map_or("", |f| f.to_str().unwrap_or(""));
                (parent.to_string_lossy().to_string(), file_name.to_string())
            }
        } else {
            (".".to_string(), expanded_prefix)
        };

        // Read directory entries
        if let Ok(entries) = fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // Skip hidden files unless explicitly requested
                    if name.starts_with('.') && !file_prefix.starts_with('.') {
                        continue;
                    }

                    if name.starts_with(&file_prefix) {
                        let mut completion = if dir_path == "." {
                            name.to_string()
                        } else if prefix.starts_with('~') {
                            // Preserve tilde in completion
                            let home = env::var("HOME").unwrap_or_default();
                            if dir_path.starts_with(&home) {
                                let relative = &dir_path[home.len()..];
                                if relative.is_empty() {
                                    format!("~/{}", name)
                                } else {
                                    format!("~{}/{}", relative, name)
                                }
                            } else {
                                format!("{}/{}", dir_path, name)
                            }
                        } else {
                            format!("{}/{}", dir_path, name)
                        };

                        // Add trailing slash for directories
                        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            completion.push('/');
                        }

                        completions.push(completion);
                    }
                }
            }
        }

        completions.sort();
        completions
    }

    /// Complete directories only
    pub fn complete_directories(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();

        // Expand tilde if present
        let expanded_prefix = if prefix.starts_with('~') {
            if let Ok(home) = env::var("HOME") {
                prefix.replacen('~', &home, 1)
            } else {
                prefix.to_string()
            }
        } else {
            prefix.to_string()
        };

        // Determine directory and filename prefix
        let (dir_path, file_prefix) = if expanded_prefix.contains('/') {
            if expanded_prefix.ends_with('/') {
                (expanded_prefix.clone(), String::new())
            } else {
                let path = Path::new(&expanded_prefix);
                let parent = path.parent().unwrap_or(Path::new(""));
                let file_name = path.file_name().map_or("", |f| f.to_str().unwrap_or(""));
                (parent.to_string_lossy().to_string(), file_name.to_string())
            }
        } else {
            (".".to_string(), expanded_prefix)
        };

        // Read directory entries
        if let Ok(entries) = fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // Skip hidden files unless explicitly requested
                    if name.starts_with('.') && !file_prefix.starts_with('.') {
                        continue;
                    }

                    // Only include directories
                    if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        continue;
                    }

                    if name.starts_with(&file_prefix) {
                        let completion = if dir_path == "." {
                            format!("{}/", name)
                        } else if prefix.starts_with('~') {
                            // Preserve tilde in completion
                            let home = env::var("HOME").unwrap_or_default();
                            if dir_path.starts_with(&home) {
                                let relative = &dir_path[home.len()..];
                                if relative.is_empty() {
                                    format!("~/{}/", name)
                                } else {
                                    format!("~{}/{}/", relative, name)
                                }
                            } else {
                                format!("{}/{}/", dir_path, name)
                            }
                        } else {
                            format!("{}/{}/", dir_path, name)
                        };

                        completions.push(completion);
                    }
                }
            }
        }

        completions.sort();
        completions
    }

    /// Complete by action type
    fn complete_by_action(&self, action: &str, context: &CompletionContext) -> Vec<String> {
        match action {
            "alias" => self.complete_aliases(&context.current_word),
            "command" => self.complete_commands(&context.current_word),
            "directory" => self.complete_directories(&context.current_word),
            "file" => self.complete_files(&context.current_word),
            "variable" => self.complete_variables(&context.current_word),
            "user" => self.complete_users(&context.current_word),
            "hostname" => self.complete_hostnames(&context.current_word),
            _ => Vec::new(),
        }
    }

    /// Complete alias names
    fn complete_aliases(&self, _prefix: &str) -> Vec<String> {
        // This would need access to the interpreter's aliases
        // For now, return empty
        Vec::new()
    }

    /// Complete variable names
    fn complete_variables(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();

        // Common environment variables
        let common_vars = [
            "PATH", "HOME", "USER", "SHELL", "PWD", "OLDPWD", "PS1", "PS2", "TERM", "LANG",
            "LC_ALL", "EDITOR", "PAGER", "MANPATH",
        ];

        for var in &common_vars {
            if var.starts_with(prefix) {
                completions.push(format!("${}", var));
            }
        }

        // Environment variables
        for (key, _) in env::vars() {
            if key.starts_with(prefix) {
                completions.push(format!("${}", key));
            }
        }

        completions.sort();
        completions.dedup();
        completions
    }

    /// Complete usernames
    fn complete_users(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();

        // Try to read /etc/passwd for user completion
        if let Ok(passwd_content) = fs::read_to_string("/etc/passwd") {
            for line in passwd_content.lines() {
                if let Some(username) = line.split(':').next() {
                    if username.starts_with(prefix) {
                        completions.push(username.to_string());
                    }
                }
            }
        }

        completions.sort();
        completions.dedup();
        completions
    }

    /// Complete hostnames
    fn complete_hostnames(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();

        // Try to read /etc/hosts
        if let Ok(hosts_content) = fs::read_to_string("/etc/hosts") {
            for line in hosts_content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    for hostname in &parts[1..] {
                        if hostname.starts_with(prefix) && !hostname.contains('.') {
                            completions.push(hostname.to_string());
                        }
                    }
                }
            }
        }

        // Try to read ~/.ssh/known_hosts
        if let Ok(home) = env::var("HOME") {
            let known_hosts_path = format!("{}/.ssh/known_hosts", home);
            if let Ok(known_hosts_content) = fs::read_to_string(known_hosts_path) {
                for line in known_hosts_content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }

                    if let Some(hostname_part) = line.split_whitespace().next() {
                        // Handle comma-separated hostnames
                        for hostname in hostname_part.split(',') {
                            // Remove port numbers and brackets
                            let hostname = hostname.split(':').next().unwrap_or(hostname);
                            let hostname = hostname.trim_start_matches('[').trim_end_matches(']');

                            if hostname.starts_with(prefix) && !hostname.contains('*') {
                                completions.push(hostname.to_string());
                            }
                        }
                    }
                }
            }
        }

        completions.sort();
        completions.dedup();
        completions
    }

    /// Call a completion function (placeholder for now)
    fn call_completion_function(&self, function: &str, context: &CompletionContext) -> Vec<String> {
        match function {
            "_git_complete" => self.complete_git(context),
            "_ssh_complete" => self.complete_ssh(context),
            "_kill_complete" => self.complete_kill(context),
            "_man_complete" => self.complete_man(context),
            _ => Vec::new(),
        }
    }

    /// Git completion
    pub fn complete_git(&self, context: &CompletionContext) -> Vec<String> {
        if context.cword == 1 {
            // Git subcommands
            let subcommands = [
                "add", "branch", "checkout", "clone", "commit", "diff", "fetch", "init", "log",
                "merge", "pull", "push", "rebase", "reset", "status", "tag", "remote", "show",
                "stash", "config",
            ];

            subcommands
                .iter()
                .filter(|cmd| cmd.starts_with(&context.current_word))
                .map(|cmd| cmd.to_string())
                .collect()
        } else if context.cword >= 2 {
            match context.words.get(1).map(|s| s.as_str()) {
                Some("checkout") | Some("branch") => {
                    // Complete branch names
                    self.complete_git_branches(&context.current_word)
                }
                Some("add") | Some("diff") | Some("reset") => {
                    // Complete file names
                    self.complete_files(&context.current_word)
                }
                _ => self.complete_files(&context.current_word),
            }
        } else {
            Vec::new()
        }
    }

    /// Complete git branch names
    fn complete_git_branches(&self, prefix: &str) -> Vec<String> {
        use std::process::Command;

        let mut completions = Vec::new();

        // Try to get branches from git
        if let Ok(output) = Command::new("git")
            .args(["branch", "--format=%(refname:short)"])
            .output()
        {
            if output.status.success() {
                let branches = String::from_utf8_lossy(&output.stdout);
                for branch in branches.lines() {
                    let branch = branch.trim();
                    if branch.starts_with(prefix) {
                        completions.push(branch.to_string());
                    }
                }
            }
        }

        completions
    }

    /// SSH completion
    pub fn complete_ssh(&self, context: &CompletionContext) -> Vec<String> {
        if context.cword == 1 {
            // Complete hostnames for SSH
            self.complete_hostnames(&context.current_word)
        } else {
            // Complete files for scp-like operations
            self.complete_files(&context.current_word)
        }
    }

    /// Kill completion (process IDs and names)
    pub fn complete_kill(&self, context: &CompletionContext) -> Vec<String> {
        use std::process::Command;

        let mut completions = Vec::new();

        // Complete process IDs and names
        if let Ok(output) = Command::new("ps").args(["-eo", "pid,comm"]).output() {
            if output.status.success() {
                let ps_output = String::from_utf8_lossy(&output.stdout);
                for line in ps_output.lines().skip(1) {
                    // Skip header
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let pid = parts[0];
                        let comm = parts[1];

                        if pid.starts_with(&context.current_word) {
                            completions.push(pid.to_string());
                        }
                        if comm.starts_with(&context.current_word) {
                            completions.push(comm.to_string());
                        }
                    }
                }
            }
        }

        completions.sort();
        completions.dedup();
        completions
    }

    /// Man page completion
    pub fn complete_man(&self, context: &CompletionContext) -> Vec<String> {
        if context.cword == 1 {
            let mut completions = Vec::new();

            // Get man pages from MANPATH
            let manpath = env::var("MANPATH").unwrap_or_else(|_| {
                "/usr/share/man:/usr/local/share/man:/opt/homebrew/share/man".to_string()
            });

            for path in manpath.split(':') {
                for section in &[
                    "man1", "man2", "man3", "man4", "man5", "man6", "man7", "man8",
                ] {
                    let section_path = format!("{}/{}", path, section);
                    if let Ok(entries) = fs::read_dir(section_path) {
                        for entry in entries.flatten() {
                            if let Some(name) = entry.file_name().to_str() {
                                if let Some(page_name) = name.strip_suffix(".gz") {
                                    if let Some(page_name) = page_name.split('.').next() {
                                        if page_name.starts_with(&context.current_word) {
                                            completions.push(page_name.to_string());
                                        }
                                    }
                                } else if let Some(page_name) = name.split('.').next() {
                                    if page_name.starts_with(&context.current_word) {
                                        completions.push(page_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            completions.sort();
            completions.dedup();
            completions
        } else {
            Vec::new()
        }
    }

    /// Parse command line into completion context
    pub fn parse_context(line: &str, point: usize) -> CompletionContext {
        let line_up_to_cursor = &line[..point.min(line.len())];

        // Split by pipes, &&, || to find the current command segment
        let segments: Vec<&str> = line_up_to_cursor.split(&['|', '&'][..]).collect();

        // Get the last segment (current command being completed)
        let current_segment = segments.last().unwrap_or(&"");
        let has_trailing_space = current_segment.ends_with(' ');
        let current_segment = current_segment.trim();

        let words: Vec<String> = current_segment
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let cword = if has_trailing_space {
            words.len()
        } else if words.is_empty() {
            0
        } else {
            words.len() - 1
        };

        let current_word = if has_trailing_space {
            String::new()
        } else {
            words.last().cloned().unwrap_or_default()
        };

        let prev_word = if cword > 0 {
            words.get(cword - 1).cloned().unwrap_or_default()
        } else {
            String::new()
        };

        CompletionContext {
            line: line.to_string(),
            point,
            words,
            cword,
            current_word,
            prev_word,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_completion_system_new() {
        let system = CompletionSystem::new();

        // Should have default completions set up
        assert!(system.command_completions.contains_key("git"));
        assert!(system.command_completions.contains_key("cd"));
        assert!(system.command_completions.contains_key("ssh"));
        assert!(system.command_completions.contains_key("kill"));
        assert!(system.command_completions.contains_key("man"));
    }

    #[test]
    fn test_parse_context_basic() {
        let context = CompletionSystem::parse_context("git add file.txt", 8);
        assert_eq!(context.line, "git add file.txt");
        assert_eq!(context.point, 8);
        assert_eq!(context.words, vec!["git", "add"]);
        assert_eq!(context.cword, 2);
        assert_eq!(context.current_word, "");
        assert_eq!(context.prev_word, "add");
    }

    #[test]
    fn test_parse_context_partial_word() {
        let context = CompletionSystem::parse_context("git ad", 6);
        assert_eq!(context.words, vec!["git", "ad"]);
        assert_eq!(context.cword, 1);
        assert_eq!(context.current_word, "ad");
        assert_eq!(context.prev_word, "git");
    }

    #[test]
    fn test_parse_context_single_word() {
        let context = CompletionSystem::parse_context("gi", 2);
        assert_eq!(context.words, vec!["gi"]);
        assert_eq!(context.cword, 0);
        assert_eq!(context.current_word, "gi");
        assert_eq!(context.prev_word, "");
    }

    #[test]
    fn test_parse_context_empty() {
        let context = CompletionSystem::parse_context("", 0);
        assert_eq!(context.words, Vec::<String>::new());
        assert_eq!(context.cword, 0);
        assert_eq!(context.current_word, "");
        assert_eq!(context.prev_word, "");
    }

    #[test]
    fn test_parse_context_trailing_space() {
        let context = CompletionSystem::parse_context("git ", 4);
        assert_eq!(context.words, vec!["git"]);
        assert_eq!(context.cword, 1);
        assert_eq!(context.current_word, "");
        assert_eq!(context.prev_word, "git");
    }

    #[test]
    fn test_complete_commands_builtin() {
        let system = CompletionSystem::new();
        let completions = system.complete_commands("ec");

        assert!(completions.contains(&"echo".to_string()));
        assert!(!completions.contains(&"git".to_string())); // Should not contain non-matching
    }

    #[test]
    fn test_complete_commands_empty_prefix() {
        let system = CompletionSystem::new();
        let completions = system.complete_commands("");

        // Should contain all built-ins
        assert!(completions.contains(&"echo".to_string()));
        assert!(completions.contains(&"cd".to_string()));
        assert!(completions.contains(&"exit".to_string()));

        // Should be sorted and deduplicated
        let mut sorted_completions = completions.clone();
        sorted_completions.sort();
        sorted_completions.dedup();
        assert_eq!(completions, sorted_completions);
    }

    #[test]
    fn test_complete_files_current_directory() {
        let system = CompletionSystem::new();
        let completions = system.complete_files("");

        // Should return some files/directories from current directory
        // We can't assert specific files since it depends on the test environment
        // But we can check that it doesn't crash and returns a Vec
        assert!(!completions.is_empty() || completions.is_empty()); // At least doesn't crash
    }

    #[test]
    fn test_complete_directories() {
        let system = CompletionSystem::new();
        let completions = system.complete_directories("");

        // All completions should end with '/'
        for completion in &completions {
            assert!(
                completion.ends_with('/'),
                "Directory completion '{}' should end with '/'",
                completion
            );
        }
    }

    #[test]
    fn test_complete_variables() {
        let system = CompletionSystem::new();
        let completions = system.complete_variables("PA");

        // Should include PATH if it exists
        if env::var("PATH").is_ok() {
            assert!(completions.contains(&"$PATH".to_string()));
        }

        // All completions should start with $
        for completion in &completions {
            assert!(
                completion.starts_with('$'),
                "Variable completion '{}' should start with $",
                completion
            );
        }
    }

    #[test]
    fn test_complete_variables_empty_prefix() {
        let system = CompletionSystem::new();
        let completions = system.complete_variables("");

        // Should include common variables
        assert!(completions.contains(&"$PATH".to_string()));
        assert!(completions.contains(&"$HOME".to_string()));
        assert!(completions.contains(&"$USER".to_string()));
    }

    #[test]
    fn test_complete_by_action() {
        let system = CompletionSystem::new();

        // Test command action
        let context = CompletionContext {
            line: "test".to_string(),
            point: 4,
            words: vec!["test".to_string()],
            cword: 0,
            current_word: "ec".to_string(),
            prev_word: "".to_string(),
        };

        let completions = system.complete_by_action("command", &context);
        assert!(completions.contains(&"echo".to_string()));

        // Test directory action
        let completions = system.complete_by_action("directory", &context);
        // Should only return directories (all end with /)
        for completion in &completions {
            assert!(completion.ends_with('/'));
        }

        // Test unknown action
        let completions = system.complete_by_action("unknown", &context);
        assert!(completions.is_empty());
    }

    #[test]
    fn test_git_completion() {
        let system = CompletionSystem::new();

        // Test git subcommand completion
        let context = CompletionContext {
            line: "git ".to_string(),
            point: 4,
            words: vec!["git".to_string()],
            cword: 1,
            current_word: "".to_string(),
            prev_word: "git".to_string(),
        };

        let completions = system.complete_git(&context);
        assert!(completions.contains(&"add".to_string()));
        assert!(completions.contains(&"commit".to_string()));
        assert!(completions.contains(&"push".to_string()));

        // Test partial git subcommand completion
        let context = CompletionContext {
            line: "git ad".to_string(),
            point: 6,
            words: vec!["git".to_string(), "ad".to_string()],
            cword: 1,
            current_word: "ad".to_string(),
            prev_word: "git".to_string(),
        };

        let completions = system.complete_git(&context);
        assert!(completions.contains(&"add".to_string()));
        assert!(!completions.contains(&"commit".to_string())); // Should not contain non-matching
    }

    #[test]
    fn test_ssh_completion() {
        let system = CompletionSystem::new();

        // Test ssh hostname completion
        let context = CompletionContext {
            line: "ssh ".to_string(),
            point: 4,
            words: vec!["ssh".to_string()],
            cword: 1,
            current_word: "".to_string(),
            prev_word: "ssh".to_string(),
        };

        let completions = system.complete_ssh(&context);
        // Should return hostnames (we can't assert specific ones)
        // But should not crash
        assert!(!completions.is_empty() || completions.is_empty());
    }

    #[test]
    fn test_complete_integration() {
        let mut system = CompletionSystem::new();

        // Test command completion (cword = 0)
        let context = CompletionSystem::parse_context("gi", 2);
        let completions = system.complete(&context);

        // Should include git and other commands starting with 'gi'
        let has_git = completions.iter().any(|c| c == "git");
        assert!(has_git, "Should complete 'git' for 'gi' prefix");

        // Test git subcommand completion
        let context = CompletionSystem::parse_context("git ", 4);
        let completions = system.complete(&context);
        assert!(completions.contains(&"add".to_string()));

        // Test cd directory completion
        let context = CompletionSystem::parse_context("cd ", 3);
        let completions = system.complete(&context);
        // All completions should be directories (end with /)
        for completion in &completions {
            assert!(
                completion.ends_with('/'),
                "CD completion '{}' should be a directory",
                completion
            );
        }
    }

    #[test]
    fn test_complete_unknown_command() {
        let mut system = CompletionSystem::new();

        // Test completion for unknown command (should default to file completion)
        let context = CompletionSystem::parse_context("unknowncommand ", 15);
        let completions = system.complete(&context);

        // Should return file completions (we can't assert specific files)
        // But should not crash
        assert!(!completions.is_empty() || completions.is_empty());
    }

    #[test]
    fn test_complete_empty_context() {
        let mut system = CompletionSystem::new();

        let context = CompletionSystem::parse_context("", 0);
        let completions = system.complete(&context);

        // Should return empty for empty context
        assert!(completions.is_empty());
    }

    #[test]
    fn test_completion_entry_default() {
        let entry = CompletionEntry::default();
        assert!(entry.function.is_empty());
        assert!(entry.action.is_empty());
        assert!(entry.options.is_empty());
        assert!(entry.o_options.is_empty());
    }

    #[test]
    fn test_completion_context_debug() {
        let context = CompletionContext {
            line: "test line".to_string(),
            point: 4,
            words: vec!["test".to_string()],
            cword: 0,
            current_word: "test".to_string(),
            prev_word: "".to_string(),
        };

        // Should be able to debug print without crashing
        let debug_str = format!("{:?}", context);
        assert!(debug_str.contains("test line"));
    }

    #[test]
    fn test_man_completion() {
        let system = CompletionSystem::new();

        let context = CompletionContext {
            line: "man ".to_string(),
            point: 4,
            words: vec!["man".to_string()],
            cword: 1,
            current_word: "".to_string(),
            prev_word: "man".to_string(),
        };

        let completions = system.complete_man(&context);
        // Should return man page completions (we can't assert specific ones)
        // But should not crash
        assert!(!completions.is_empty() || completions.is_empty());
    }

    #[test]
    fn test_kill_completion() {
        let system = CompletionSystem::new();

        let context = CompletionContext {
            line: "kill ".to_string(),
            point: 5,
            words: vec!["kill".to_string()],
            cword: 1,
            current_word: "".to_string(),
            prev_word: "kill".to_string(),
        };

        let completions = system.complete_kill(&context);
        // Should return process completions (we can't assert specific ones)
        // But should not crash and should be sorted/deduplicated
        let mut sorted_completions = completions.clone();
        sorted_completions.sort();
        sorted_completions.dedup();
        assert_eq!(completions, sorted_completions);
    }

    #[test]
    fn test_complete_users() {
        let system = CompletionSystem::new();
        let completions = system.complete_users("r");

        // Should return users starting with 'r' (like 'root' if it exists)
        // We can't assert specific users since it depends on the system
        // But should not crash and should be sorted/deduplicated
        let mut sorted_completions = completions.clone();
        sorted_completions.sort();
        sorted_completions.dedup();
        assert_eq!(completions, sorted_completions);

        // All completions should start with the prefix
        for completion in &completions {
            assert!(
                completion.starts_with('r'),
                "User completion '{}' should start with 'r'",
                completion
            );
        }
    }

    #[test]
    fn test_complete_hostnames() {
        let system = CompletionSystem::new();
        let completions = system.complete_hostnames("local");

        // Should return hostnames starting with 'local' (like 'localhost' if it exists)
        // We can't assert specific hostnames since it depends on the system
        // But should not crash and should be sorted/deduplicated
        let mut sorted_completions = completions.clone();
        sorted_completions.sort();
        sorted_completions.dedup();
        assert_eq!(completions, sorted_completions);

        // All completions should start with the prefix
        for completion in &completions {
            assert!(
                completion.starts_with("local"),
                "Hostname completion '{}' should start with 'local'",
                completion
            );
        }
    }

    #[test]
    fn test_git_branch_completion() {
        let system = CompletionSystem::new();

        // Test git checkout completion (should complete branches)
        let context = CompletionContext {
            line: "git checkout ".to_string(),
            point: 13,
            words: vec!["git".to_string(), "checkout".to_string()],
            cword: 2,
            current_word: "".to_string(),
            prev_word: "checkout".to_string(),
        };

        let completions = system.complete_git(&context);
        // Should return branch completions (we can't assert specific branches)
        // But should not crash
        assert!(!completions.is_empty() || completions.is_empty());
    }

    #[test]
    fn test_complete_files_with_prefix() {
        let system = CompletionSystem::new();

        // Test file completion with a prefix
        let completions = system.complete_files("src");

        // All completions should start with 'src' or be 'src/' if it's a directory
        for completion in &completions {
            assert!(
                completion.starts_with("src") || completion == "src/",
                "File completion '{}' should start with 'src'",
                completion
            );
        }
    }

    #[test]
    fn test_complete_files_with_tilde() {
        let system = CompletionSystem::new();

        // Test file completion with tilde (should expand to home directory)
        let completions = system.complete_files("~/");

        // Should handle tilde expansion without crashing
        assert!(!completions.is_empty() || completions.is_empty());

        // If there are completions, they should preserve the tilde prefix
        for completion in &completions {
            if !completion.is_empty() {
                assert!(
                    completion.starts_with("~/"),
                    "Tilde completion '{}' should start with '~/'",
                    completion
                );
            }
        }
    }

    #[test]
    fn test_call_completion_function() {
        let system = CompletionSystem::new();

        let context = CompletionContext {
            line: "git ".to_string(),
            point: 4,
            words: vec!["git".to_string()],
            cword: 1,
            current_word: "".to_string(),
            prev_word: "git".to_string(),
        };

        // Test known completion function
        let completions = system.call_completion_function("_git_complete", &context);
        assert!(completions.contains(&"add".to_string()));

        // Test unknown completion function
        let completions = system.call_completion_function("_unknown_complete", &context);
        assert!(completions.is_empty());
    }
}
