use crate::parser::Node;
use crate::parser::RedirectKind;

/// Formatter for shell scripts
pub struct Formatter {
    indent_level: usize,
    indent_str: String,
}

impl Formatter {
    pub fn new(indent_str: &str) -> Self {
        Self {
            indent_level: 0,
            indent_str: indent_str.to_string(),
        }
    }

    #[inline]
    pub fn set_indent_level(&mut self, level: usize) {
        self.indent_level = level;
    }

    pub fn indent(&self) -> String {
        self.indent_str.repeat(self.indent_level)
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
            Node::VariableAssignmentCommand { .. } => todo!(),
            Node::ExtGlobPattern { .. } => todo!(),
            &Node::IfStatement { .. } | &Node::ElifBranch { .. } | &Node::ElseBranch { .. } => {
                todo!()
            }
        }
    }
}
