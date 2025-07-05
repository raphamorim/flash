/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use crate::lexer::Lexer;
use crate::parser::Node;
use crate::parser::Parser;
use crate::parser::RedirectKind;

/// Configuration options for the shell script formatter
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Indentation string (spaces or tabs)
    pub indent_str: String,
    /// Shell variant (posix, bash, etc.)
    pub shell_variant: ShellVariant,
    /// Place binary operator at the beginning of the next line
    pub binary_next_line: bool,
    /// Indent case statements
    pub switch_case_indent: bool,
    /// Add spaces around redirect operators
    pub space_redirects: bool,
    /// Keep existing padding/formatting where possible
    pub keep_padding: bool,
    /// Place function opening brace on the next line
    pub function_next_line: bool,
    /// Avoid splitting complex commands into multiple lines
    pub never_split: bool,
    /// Only format if necessary (leave simple commands as they are)
    pub format_if_needed: bool,
}

#[inline]
fn parse_str(input: &str) -> Node {
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    parser.parse_script()
}

/// Supported shell variants
#[derive(Debug, Clone, PartialEq)]
pub enum ShellVariant {
    Posix,
    Bash,
    Ksh,
    Zsh,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            indent_str: "    ".to_string(), // 4 spaces by default
            shell_variant: ShellVariant::Posix,
            binary_next_line: false,
            switch_case_indent: false,
            space_redirects: false,
            keep_padding: false,
            function_next_line: false,
            never_split: false,
            format_if_needed: true,
        }
    }
}

impl FormatterConfig {
    pub fn from_config_str(config: &str) -> Self {
        let mut formatter_config = FormatterConfig::default();

        for line in config.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if let Some((key, value)) = parse_config_line(line) {
                match key {
                    "indent_style" => {
                        if value == "tab" {
                            formatter_config.indent_str = "\t".to_string();
                        } else if value == "space" {
                            // Need indent_size to determine the number of spaces
                        }
                    }
                    "indent_size" => {
                        if let Ok(size) = value.parse::<usize>() {
                            if formatter_config.indent_str != "\t" {
                                formatter_config.indent_str = " ".repeat(size);
                            }
                        }
                    }
                    "shell_variant" => {
                        formatter_config.shell_variant = match value {
                            "posix" => ShellVariant::Posix,
                            "bash" => ShellVariant::Bash,
                            "ksh" => ShellVariant::Ksh,
                            "zsh" => ShellVariant::Zsh,
                            _ => ShellVariant::Posix,
                        };
                    }
                    "binary_next_line" => {
                        formatter_config.binary_next_line = value == "true";
                    }
                    "switch_case_indent" => {
                        formatter_config.switch_case_indent = value == "true";
                    }
                    "space_redirects" => {
                        formatter_config.space_redirects = value == "true";
                    }
                    "keep_padding" => {
                        formatter_config.keep_padding = value == "true";
                    }
                    "function_next_line" => {
                        formatter_config.function_next_line = value == "true";
                    }
                    "never_split" => {
                        formatter_config.never_split = value == "true";
                    }
                    "format_if_needed" => {
                        formatter_config.format_if_needed = value == "true";
                    }
                    _ => {}
                }
            }
        }

        formatter_config
    }
}

/// Parse a single config line into a (key, value) pair
fn parse_config_line(line: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() == 2 {
        Some((parts[0].trim(), parts[1].trim()))
    } else {
        None
    }
}

/// Formatter for shell scripts
pub struct Formatter {
    indent_level: usize,
    config: FormatterConfig,
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter {
    /// Create a new formatter with default configuration
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            config: FormatterConfig::default(),
        }
    }

    /// Create a new formatter with a specific configuration
    pub fn with_config(config: FormatterConfig) -> Self {
        Self {
            indent_level: 0,
            config,
        }
    }

    /// Create a new formatter from an EditorConfig-like string
    pub fn from_config_str(config: &str) -> Self {
        Self {
            indent_level: 0,
            config: FormatterConfig::from_config_str(config),
        }
    }

    #[inline]
    pub fn set_indent_level(&mut self, level: usize) {
        self.indent_level = level;
    }

    pub fn indent(&self) -> String {
        self.config.indent_str.repeat(self.indent_level)
    }

    /// Check if node needs formatting
    fn needs_formatting(&self, node: &Node) -> bool {
        if !self.config.format_if_needed {
            // Always format if the config flag is disabled
            return true;
        }

        match node {
            // Simple command with no redirects doesn't need formatting
            Node::Command {
                args, redirects, ..
            } if args.is_empty() && redirects.is_empty() => false,

            // Simple single command pipeline doesn't need formatting
            Node::Pipeline { commands }
                if commands.len() == 1 && !self.needs_formatting(&commands[0]) =>
            {
                false
            }

            // Single statement list with no operators doesn't need formatting
            Node::List {
                statements,
                operators,
            } if statements.len() == 1
                && operators.is_empty()
                && !self.needs_formatting(&statements[0]) =>
            {
                false
            }

            // Simple string literal doesn't need formatting
            Node::StringLiteral(_) => false,
            Node::SingleQuotedString(_) => false,

            // Simple comment doesn't need formatting
            Node::Comment(_) => false,

            // Everything else needs formatting
            _ => true,
        }
    }

    /// Format a string input by parsing it into a Node and then formatting
    pub fn format_str(&mut self, input: &str) -> String {
        // Use the parser to parse the input string
        let node = parse_str(input);
        // If the node doesn't need formatting, return the original string
        if !self.needs_formatting(&node) {
            input.to_string()
        } else {
            // Otherwise format the node
            self.format(&node)
        }
    }

    pub fn format(&mut self, node: &Node) -> String {
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
                    let redirect_op = match redirect.kind {
                        RedirectKind::Input => "<",
                        RedirectKind::Output => ">",
                        RedirectKind::Append => ">>",
                        RedirectKind::HereDoc => "<<",
                        RedirectKind::HereDocDash => "<<-",
                        RedirectKind::HereString => "<<<",
                        RedirectKind::InputDup => "<&",
                        RedirectKind::OutputDup => ">&",
                    };

                    if self.config.space_redirects {
                        result.push_str(&format!(" {} ", redirect_op));
                    } else {
                        result.push_str(&format!(" {}", redirect_op));
                        if !redirect.file.starts_with('&') {
                            // Don't add space for &2 etc.
                            result.push(' ');
                        }
                    }

                    result.push_str(&redirect.file);
                }

                result
            }
            Node::Pipeline { commands } => {
                if commands.is_empty() {
                    return String::new();
                }

                if self.config.binary_next_line && commands.len() > 1 && !self.config.never_split {
                    let mut result = String::new();

                    // First command
                    result.push_str(&self.format(&commands[0]));

                    // Remaining commands with pipe at start of next line
                    for cmd in &commands[1..] {
                        result.push_str(" \\\n");
                        result.push_str(&self.config.indent_str); // Add one level of indentation
                        result.push_str("| ");

                        // Format command and remove its indent since we already added it
                        let cmd_str = self.format(cmd);
                        result.push_str(cmd_str.trim_start());
                    }

                    result
                } else {
                    let mut parts = Vec::new();
                    for cmd in commands {
                        parts.push(self.format(cmd));
                    }
                    parts.join(" | ")
                }
            }
            Node::List {
                statements,
                operators,
            } => {
                if statements.is_empty() {
                    return String::new();
                }

                let mut result = String::new();

                for (i, statement) in statements.iter().enumerate() {
                    if i > 0 {
                        let operator = &operators[i - 1];

                        if operator == "\n" {
                            result.push('\n');
                            result.push('\n');
                        } else if self.config.binary_next_line
                            && !self.config.never_split
                            && (operator == "&&" || operator == "||")
                        {
                            result.push_str(" \\\n");
                            result.push_str(&self.config.indent_str); // Add one level of indentation
                            result.push_str(operator);
                            result.push(' ');
                        } else {
                            result.push(' ');
                            result.push_str(operator);
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
                        result.push_str(&self.format(value));
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
            Node::SingleQuotedString(value) => {
                let mut result = String::new();
                result.push('\'');
                result.push_str(value);
                result.push('\'');
                result
            }
            Node::Subshell { list } => {
                let mut result = self.indent();
                result.push('(');

                if !self.config.never_split {
                    result.push('\n');

                    self.indent_level += 1;
                    result.push_str(&self.format(list));
                    self.indent_level -= 1;

                    result.push('\n');
                    result.push_str(&self.indent());
                    result.push(')');
                } else {
                    result.push(' ');

                    let list_str = self.format(list);
                    result.push_str(list_str.trim());

                    result.push_str(" )");
                }

                result
            }
            Node::Comment(comment) => {
                let mut result = self.indent();
                if !comment.starts_with('#') {
                    result.push('#');
                }
                result.push_str(comment);
                result
            }
            Node::ExtGlobPattern {
                operator,
                patterns,
                suffix,
            } => {
                let mut result = self.indent();

                // Format as: ?(pattern1|pattern2)suffix
                result.push(*operator);
                result.push('(');

                for (i, pattern) in patterns.iter().enumerate() {
                    if i > 0 {
                        result.push('|');
                    }
                    result.push_str(pattern);
                }

                result.push(')');
                result.push_str(suffix);

                result
            }
            Node::IfStatement {
                condition,
                consequence,
                alternative,
            } => {
                let mut result = self.indent();
                result.push_str("if ");

                // Format the condition
                let condition_str = self.format(condition);
                result.push_str(condition_str.trim_start());

                result.push_str("; then");

                if !self.config.never_split {
                    result.push('\n');

                    // Format the consequence with increased indent
                    self.indent_level += 1;
                    result.push_str(&self.format(consequence));
                    self.indent_level -= 1;
                } else {
                    result.push(' ');

                    // Format the consequence on the same line
                    let consequence_str = self.format(consequence);
                    result.push_str(consequence_str.trim_start());
                }

                // Format the alternative if it exists
                if let Some(alt) = alternative {
                    match &**alt {
                        Node::ElifBranch { .. } => {
                            if !self.config.never_split {
                                result.push('\n');
                            } else {
                                result.push(' ');
                            }

                            let alt_str = self.format(alt);
                            result.push_str(&alt_str);
                        }
                        Node::ElseBranch { .. } => {
                            if !self.config.never_split {
                                result.push('\n');
                            } else {
                                result.push(' ');
                            }

                            let alt_str = self.format(alt);
                            result.push_str(&alt_str);
                        }
                        _ => {
                            if !self.config.never_split {
                                result.push('\n');
                                result.push_str(&self.indent());
                            } else {
                                result.push(' ');
                            }

                            result.push_str("else");

                            if !self.config.never_split {
                                result.push('\n');

                                self.indent_level += 1;
                                result.push_str(&self.format(alt));
                                self.indent_level -= 1;

                                result.push('\n');
                                result.push_str(&self.indent());
                            } else {
                                result.push(' ');

                                let alt_str = self.format(alt);
                                result.push_str(alt_str.trim_start());
                                result.push(' ');
                            }

                            result.push_str("fi");
                        }
                    }
                } else {
                    if !self.config.never_split {
                        result.push('\n');
                        result.push_str(&self.indent());
                    } else {
                        result.push(' ');
                    }

                    result.push_str("fi");
                }

                result
            }
            Node::ElifBranch {
                condition,
                consequence,
            } => {
                let mut result = self.indent();
                result.push_str("elif ");

                // Format the condition
                let condition_str = self.format(condition);
                result.push_str(condition_str.trim_start());

                result.push_str("; then");

                if !self.config.never_split {
                    result.push('\n');

                    // Format the consequence with increased indent
                    self.indent_level += 1;
                    result.push_str(&self.format(consequence));
                    self.indent_level -= 1;
                } else {
                    result.push(' ');

                    // Format the consequence on the same line
                    let consequence_str = self.format(consequence);
                    result.push_str(consequence_str.trim_start());
                }

                result
            }
            Node::ElseBranch { consequence } => {
                let mut result = self.indent();
                result.push_str("else");

                if !self.config.never_split {
                    result.push('\n');

                    // Format the consequence with increased indent
                    self.indent_level += 1;
                    result.push_str(&self.format(consequence));
                    self.indent_level -= 1;

                    result.push('\n');
                    result.push_str(&self.indent());
                } else {
                    result.push(' ');

                    // Format the consequence on the same line
                    let consequence_str = self.format(consequence);
                    result.push_str(consequence_str.trim_start());
                    result.push(' ');
                }

                result.push_str("fi");

                result
            }
            Node::CaseStatement {
                expression,
                patterns,
            } => {
                let mut result = self.indent();
                result.push_str("case ");

                // Format the expression
                let expr_str = self.format(expression);
                result.push_str(expr_str.trim_start());

                result.push_str(" in");

                if !self.config.never_split {
                    result.push('\n');
                }

                // Format each pattern
                for pattern in patterns {
                    if !self.config.never_split {
                        self.indent_level += 1;
                        result.push_str(&self.indent());
                        self.indent_level -= 1;
                    } else {
                        result.push(' ');
                    }

                    // Format pattern list
                    result.push_str(&pattern.patterns.join(" | "));
                    result.push(')');

                    if !self.config.never_split {
                        result.push('\n');

                        // Format the body with increased indent
                        self.indent_level += 1;
                        result.push_str(&self.format(&pattern.body));
                        self.indent_level -= 1;

                        result.push('\n');
                        self.indent_level += 1;
                        result.push_str(&self.indent());
                        self.indent_level -= 1;
                        result.push_str(";;");
                        result.push('\n');
                    } else {
                        result.push(' ');
                        let body_str = self.format(&pattern.body);
                        result.push_str(body_str.trim_start());
                        result.push_str(" ;; ");
                    }
                }

                if !self.config.never_split {
                    result.push_str(&self.indent());
                } else {
                    result.push(' ');
                }
                result.push_str("esac");

                result
            }
            _ => "".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::formatter::{Formatter, FormatterConfig, ShellVariant};
    use crate::parser::{Node, Redirect, RedirectKind};

    #[test]
    fn test_simple_command_not_formatted() {
        let mut formatter = Formatter::new();

        // Simple command should not be formatted
        let input = "echo hello";
        let output = formatter.format_str(input);
        assert_eq!(output, input, "Simple command should not be formatted");

        // Another simple command
        let input = "ls -la";
        let output = formatter.format_str(input);
        assert_eq!(
            output, input,
            "Simple command with args should not be formatted"
        );
    }

    #[test]
    fn test_complex_command_is_formatted() {
        let mut formatter = Formatter::new();

        // If statement should be formatted
        let input = "if [ -f file.txt ]; then echo found; else echo not found; fi";
        let output = formatter.format_str(input);
        assert_ne!(output, input, "Complex command should be formatted");
        assert!(
            output.contains("\n"),
            "Formatted output should contain newlines"
        );
    }

    #[test]
    fn test_pipeline_formatting() {
        let mut formatter = Formatter::new();

        // Simple pipeline should not be formatted
        let input = "echo hello | grep hello";
        let output = formatter.format_str(input);
        assert_eq!(output, input, "Simple pipeline should not be formatted");

        // Complex pipeline should be formatted if binary_next_line is true
        let config = FormatterConfig {
            binary_next_line: true,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);

        let input = "echo hello | grep hello | wc -l";
        let output = formatter.format_str(input);
        assert_ne!(
            output, input,
            "Complex pipeline should be formatted with binary_next_line=true"
        );
        assert!(
            output.contains("\\\n"),
            "Should have continuation character"
        );
    }

    #[test]
    fn test_conditional_command_formatting() {
        let mut formatter = Formatter::new();

        // Simple conditional should not be formatted
        let input = "[ -f file.txt ] && echo found";
        let output = formatter.format_str(input);
        assert_eq!(output, input, "Simple conditional should not be formatted");

        // Conditional with binary_next_line should be formatted
        let config = FormatterConfig {
            binary_next_line: true,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);

        let input = "[ -f file.txt ] && echo found || echo not found";
        let output = formatter.format_str(input);
        assert_ne!(
            output, input,
            "Conditional should be formatted with binary_next_line=true"
        );
        assert!(
            output.contains("\\\n"),
            "Should have continuation character"
        );
    }

    #[test]
    fn test_if_statement_formatting() {
        let mut formatter = Formatter::new();

        let input = "if [ -f file.txt ]; then echo found; else echo not found; fi";
        let output = formatter.format_str(input);

        // Should have a newline after "then"
        assert!(
            output.contains("then\n"),
            "Should have newline after 'then'"
        );
        // Should have a newline after indented content
        assert!(
            output.matches("\n").count() >= 3,
            "Should have multiple newlines in formatted output"
        );
    }

    #[test]
    fn test_subshell_formatting() {
        let mut formatter = Formatter::new();

        let input = "(cd /tmp && echo hello)";
        let output = formatter.format_str(input);

        // Subshell should be formatted with newlines
        assert!(
            output.contains("(\n"),
            "Should have newline after opening parenthesis"
        );
        assert!(
            output.contains("\n)"),
            "Should have newline before closing parenthesis"
        );
    }

    #[test]
    fn test_never_split_option() {
        let config = FormatterConfig {
            never_split: true,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);

        let input = "if [ -f file.txt ]; then echo found; else echo not found; fi";
        let output = formatter.format_str(input);

        // Should not have newlines when never_split is true
        assert!(
            !output.contains("\n"),
            "Should not have newlines with never_split=true"
        );
    }

    #[test]
    fn test_multiline_script() {
        let mut formatter = Formatter::new();

        let input = r#"#!/bin/bash
echo "Starting script"
if [ -f file.txt ]; then
    echo "File found"
    cat file.txt
else
    echo "File not found"
    touch file.txt
fi
echo "Done"
"#;
        let output = formatter.format_str(input);

        // Multiline script should be formatted
        assert_ne!(output, input, "Multiline script should be formatted");

        // Should preserve shebang
        assert!(
            output.starts_with("#!/bin/bash"),
            "Should preserve shebang line"
        );
    }

    #[test]
    fn test_comment_preservation() {
        let mut formatter = Formatter::new();

        let input = "# This is a comment\necho hello  # Inline comment";
        let output = formatter.format_str(input);

        // Comments should be preserved
        assert!(
            output.contains("# This is a comment"),
            "Should preserve standalone comment"
        );
        assert!(
            output.contains("# Inline comment"),
            "Should preserve inline comment"
        );
    }

    #[test]
    fn test_disable_format_if_needed() {
        let config = FormatterConfig {
            format_if_needed: true,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);

        // Even simple command should be formatted when format_if_needed is false
        let input = "echo hello";
        let output = formatter.format_str(input);

        // Should add indentation even to simple commands
        assert_eq!(
            output, "echo hello",
            "Simple command formatting should be consistent"
        );
    }

    #[test]
    fn test_space_redirects_option() {
        let config = FormatterConfig {
            space_redirects: true,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);

        let input = "echo hello>file.txt";
        let output = formatter.format_str(input);

        // Should add spaces around redirect operators
        assert!(
            output.contains(" > "),
            "Should add spaces around redirect operator"
        );
    }

    #[test]
    fn test_quoted_arguments() {
        let mut formatter = Formatter::new();

        let input = r#"echo "hello world""#;
        let output = formatter.format_str(input);

        // Should preserve quotes
        assert!(
            output.contains(r#""hello world""#),
            "Should preserve quoted string"
        );
    }

    #[test]
    fn test_variable_assignment() {
        let mut formatter = Formatter::new();

        let input = "VAR=value echo $VAR";
        let output = formatter.format_str(input);

        // Variable assignment before command should be preserved
        assert!(
            output.contains("VAR=value"),
            "Should preserve variable assignment"
        );
    }

    #[test]
    fn test_bash_specific_features() {
        let config = FormatterConfig {
            shell_variant: ShellVariant::Bash,
            ..Default::default()
        };
        let mut formatter = Formatter::with_config(config);

        let input = "echo {1..5}";
        let output = formatter.format_str(input);

        // Should preserve Bash-specific syntax
        assert_eq!(output, input, "Should preserve Bash brace expansion");
    }

    #[test]
    fn test_command_substitution() {
        let mut formatter = Formatter::new();

        let input = "echo $(date)";
        let output = formatter.format_str(input);

        // Command substitution should be preserved
        assert_eq!(output, input, "Should preserve simple command substitution");

        // Complex command substitution should be formatted
        let input = "echo $(if [ -f file.txt ]; then echo found; else echo not found; fi)";
        let output = formatter.format_str(input);
        assert_ne!(
            output, input,
            "Complex command substitution should be formatted"
        );
    }

    #[test]
    fn test_formatter_config_from_string() {
        let config_str = r#"
        indent_style = space
        indent_size = 2
        shell_variant = bash
        binary_next_line = true
        switch_case_indent = true
        space_redirects = true
        keep_padding = false
        function_next_line = true
        never_split = false
        "#;

        let config = FormatterConfig::from_config_str(config_str);
        assert_eq!(config.indent_str, "  ");
        assert_eq!(config.shell_variant, ShellVariant::Bash);
        assert!(config.binary_next_line);
        assert!(config.switch_case_indent);
        assert!(config.space_redirects);
        assert!(!config.keep_padding);
        assert!(config.function_next_line);
        assert!(!config.never_split);
    }

    #[test]
    fn test_formatter_with_tab_indent() {
        let config_str = "indent_style = tab";
        let config = FormatterConfig::from_config_str(config_str);
        assert_eq!(config.indent_str, "\t");
    }

    #[test]
    fn test_formatter_config_comments_and_empty_lines() {
        let config_str = r#"
        # This is a comment
        indent_style = space
        
        indent_size = 4
        # Another comment
        "#;

        let config = FormatterConfig::from_config_str(config_str);
        assert_eq!(config.indent_str, "    ");
    }

    #[test]
    fn test_format_command() {
        let mut formatter = Formatter::new();

        let node = Node::Command {
            name: "echo".to_string(),
            args: vec!["hello".to_string(), "world".to_string()],
            redirects: vec![],
        };

        assert_eq!(formatter.format(&node), "echo hello world");
    }

    #[test]
    fn test_format_command_with_quoted_args() {
        let mut formatter = Formatter::new();

        let node = Node::Command {
            name: "echo".to_string(),
            args: vec!["hello world".to_string(), "test".to_string()],
            redirects: vec![],
        };

        assert_eq!(formatter.format(&node), "echo \"hello world\" test");
    }

    #[test]
    fn test_format_command_with_redirects_default() {
        let mut formatter = Formatter::new();

        let node = Node::Command {
            name: "cat".to_string(),
            args: vec!["file.txt".to_string()],
            redirects: vec![Redirect {
                kind: RedirectKind::Output,
                file: "output.txt".to_string(),
            }],
        };

        assert_eq!(formatter.format(&node), "cat file.txt > output.txt");
    }

    #[test]
    fn test_format_command_with_redirects_spaced() {
        let config_str = "space_redirects = true";
        let mut formatter = Formatter::from_config_str(config_str);

        let node = Node::Command {
            name: "cat".to_string(),
            args: vec!["file.txt".to_string()],
            redirects: vec![Redirect {
                kind: RedirectKind::Output,
                file: "output.txt".to_string(),
            }],
        };

        assert_eq!(formatter.format(&node), "cat file.txt > output.txt");
    }

    #[test]
    fn test_format_pipeline_normal() {
        let mut formatter = Formatter::new();

        let node = Node::Pipeline {
            commands: vec![
                Node::Command {
                    name: "cat".to_string(),
                    args: vec!["file.txt".to_string()],
                    redirects: vec![],
                },
                Node::Command {
                    name: "grep".to_string(),
                    args: vec!["pattern".to_string()],
                    redirects: vec![],
                },
            ],
        };

        assert_eq!(formatter.format(&node), "cat file.txt | grep pattern");
    }

    #[test]
    fn test_format_pipeline_with_binary_next_line() {
        let config_str = "binary_next_line = true";
        let mut formatter = Formatter::from_config_str(config_str);

        let node = Node::Pipeline {
            commands: vec![
                Node::Command {
                    name: "cat".to_string(),
                    args: vec!["file.txt".to_string()],
                    redirects: vec![],
                },
                Node::Command {
                    name: "grep".to_string(),
                    args: vec!["pattern".to_string()],
                    redirects: vec![],
                },
            ],
        };

        assert_eq!(
            formatter.format(&node),
            "cat file.txt \\\n    | grep pattern"
        );
    }

    #[test]
    fn test_format_pipeline_with_never_split() {
        let config_str = "binary_next_line = true\nnever_split = true";
        let mut formatter = Formatter::from_config_str(config_str);

        let node = Node::Pipeline {
            commands: vec![
                Node::Command {
                    name: "cat".to_string(),
                    args: vec!["file.txt".to_string()],
                    redirects: vec![],
                },
                Node::Command {
                    name: "grep".to_string(),
                    args: vec!["pattern".to_string()],
                    redirects: vec![],
                },
            ],
        };

        assert_eq!(formatter.format(&node), "cat file.txt | grep pattern");
    }

    #[test]
    fn test_format_list() {
        let mut formatter = Formatter::new();

        let node = Node::List {
            statements: vec![
                Node::Command {
                    name: "echo".to_string(),
                    args: vec!["first".to_string()],
                    redirects: vec![],
                },
                Node::Command {
                    name: "echo".to_string(),
                    args: vec!["second".to_string()],
                    redirects: vec![],
                },
            ],
            operators: vec![";".to_string()],
        };

        assert_eq!(formatter.format(&node), "echo first ; echo second");
    }

    #[test]
    fn test_format_list_with_logical_operators_and_binary_next_line() {
        let config_str = "binary_next_line = true";
        let mut formatter = Formatter::from_config_str(config_str);

        let node = Node::List {
            statements: vec![
                Node::Command {
                    name: "test".to_string(),
                    args: vec!["-f".to_string(), "file.txt".to_string()],
                    redirects: vec![],
                },
                Node::Command {
                    name: "echo".to_string(),
                    args: vec!["Found".to_string()],
                    redirects: vec![],
                },
            ],
            operators: vec!["&&".to_string()],
        };

        assert_eq!(
            formatter.format(&node),
            "test -f file.txt \\\n    && echo Found"
        );
    }

    #[test]
    fn test_format_list_with_newlines() {
        let mut formatter = Formatter::new();

        let node = Node::List {
            statements: vec![
                Node::Command {
                    name: "echo".to_string(),
                    args: vec!["first".to_string()],
                    redirects: vec![],
                },
                Node::Command {
                    name: "echo".to_string(),
                    args: vec!["second".to_string()],
                    redirects: vec![],
                },
            ],
            operators: vec!["\n".to_string()],
        };

        assert_eq!(formatter.format(&node), "echo first\n\necho second");
    }

    #[test]
    fn test_format_assignment() {
        let mut formatter = Formatter::new();

        let node = Node::Assignment {
            name: "VAR".to_string(),
            value: Box::new(Node::StringLiteral("value".to_string())),
        };

        assert_eq!(formatter.format(&node), "VAR=value");
    }

    #[test]
    fn test_format_assignment_with_spaces() {
        let mut formatter = Formatter::new();

        let node = Node::Assignment {
            name: "VAR".to_string(),
            value: Box::new(Node::StringLiteral("hello world".to_string())),
        };

        assert_eq!(formatter.format(&node), "VAR=\"hello world\"");
    }

    #[test]
    fn test_format_command_substitution() {
        let mut formatter = Formatter::new();

        let node = Node::CommandSubstitution {
            command: Box::new(Node::Command {
                name: "echo".to_string(),
                args: vec!["hello".to_string()],
                redirects: vec![],
            }),
        };

        assert_eq!(formatter.format(&node), "$(echo hello)");
    }

    #[test]
    fn test_format_assignment_with_command_substitution() {
        let mut formatter = Formatter::new();

        let node = Node::Assignment {
            name: "VAR".to_string(),
            value: Box::new(Node::CommandSubstitution {
                command: Box::new(Node::Command {
                    name: "echo".to_string(),
                    args: vec!["hello".to_string()],
                    redirects: vec![],
                }),
            }),
        };

        assert_eq!(formatter.format(&node), "VAR=$(echo hello)");
    }

    #[test]
    fn test_format_string_literal() {
        let mut formatter = Formatter::new();

        let node = Node::StringLiteral("hello".to_string());

        assert_eq!(formatter.format(&node), "hello");
    }

    #[test]
    fn test_format_string_literal_with_spaces() {
        let mut formatter = Formatter::new();

        let node = Node::StringLiteral("hello world".to_string());

        assert_eq!(formatter.format(&node), "\"hello world\"");
    }

    #[test]
    fn test_format_subshell_default() {
        let mut formatter = Formatter::new();

        let node = Node::Subshell {
            list: Box::new(Node::Command {
                name: "echo".to_string(),
                args: vec!["hello".to_string()],
                redirects: vec![],
            }),
        };

        assert_eq!(formatter.format(&node), "(\n    echo hello\n)");
    }

    #[test]
    fn test_format_subshell_never_split() {
        let config_str = "never_split = true";
        let mut formatter = Formatter::from_config_str(config_str);

        let node = Node::Subshell {
            list: Box::new(Node::Command {
                name: "echo".to_string(),
                args: vec!["hello".to_string()],
                redirects: vec![],
            }),
        };

        assert_eq!(formatter.format(&node), "( echo hello )");
    }

    #[test]
    fn test_format_comment() {
        let mut formatter = Formatter::new();

        let node = Node::Comment(" This is a comment".to_string());

        assert_eq!(formatter.format(&node), "# This is a comment");
    }

    #[test]
    fn test_format_comment_with_hash() {
        let mut formatter = Formatter::new();

        let node = Node::Comment("# This is a comment".to_string());

        assert_eq!(formatter.format(&node), "# This is a comment");
    }
}
