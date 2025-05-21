# Flash (work in progress)

*A shell parser, formatter, and interpreter written in Rust.*

Flash is a fast, extensible, and hackable toolkit for working with POSIX-style shell scripts. It includes a parser, formatter, and interpreter built from scratch in Rust. Flash understands real-world shell syntax and provides structured AST access for static analysis, tooling, and transformation.

> Inspired by [mvdan/sh](https://pkg.go.dev/mvdan.cc/sh/v3/syntax), but engineered from the ground up with performance and extensibility in mind.

## Motivation

Flash was created to serve two main purposes: as a learning project to better understand shell parsing and syntax, and as a tool for testing and embedding within the [Rio terminal emulator](https://github.com/raphamorim/rio/), a GPU-accelerated terminal written in Rust.

## Feature Coverage

This table outlines the supported features of POSIX Shell and Bash. Use it to track what your **Flash** parser and interpreter implementation in Rust supports.

Legends:

- ✅ fully supported.
- ⚠️ only supported in parser and formatter.
- ❌ not supported.

| Category              | Functionality / Feature                         | POSIX Shell | Bash | Flash | Notes |
|-----------------------|--------------------------------------------------|-------------|------|------|-------|
| **Basic Syntax**      | Variable assignment                             | ✅          | ✅   | ✅  | `VAR=value` |
|                       | Command substitution                            | ✅          | ✅   | ✅  | `$(cmd)` and `` `cmd` `` |
|                       | Arithmetic substitution                         | ❌          | ✅   | ❌  | `$((expr))` |
|                       | Comments (`#`)                                  | ✅          | ✅   | ✅  | |
|                       | Quoting (`'`, "", `\`)                          | ✅          | ✅   | ✅  | |
|                       | Globbing (`*`, `?`, `[...]`)                    | ✅          | ✅   | ❌  | |
| **Control Structures**| `if` / `else` / `elif`                          | ✅          | ✅   | ❌  | |
|                       | `case` / `esac`                                 | ✅          | ✅   | ❌  | |
|                       | `for` loops                                     | ✅          | ✅   | ❌  | |
|                       | `while`, `until` loops                          | ✅          | ✅   | ❌  | |
|                       | `select` loop                                   | ❌          | ✅   | ❌  | |
|                       | `[[ ... ]]` test command                        | ❌          | ✅   | ❌  | Extended test |
| **Functions**         | Function definition (`name() {}`)               | ✅          | ✅   | ✅  | |
|                       | `function` keyword                              | ❌          | ✅   | ✅  | Bash-specific |
| **I/O Redirection**   | Output/input redirection (`>`, `<`, `>>`)       | ✅          | ✅   | ✅  | |
|                       | Here documents (`<<`, `<<-`)                    | ✅          | ✅   | ❌  | |
|                       | Here strings (`<<<`)                            | ❌          | ✅   | ❌  | |
|                       | File descriptor duplication (`>&`, `<&`)        | ✅          | ✅   | ❌  | |
| **Job Control**       | Background execution (`&`)                      | ✅          | ✅   | ❌  | |
|                       | Job control commands (`fg`, `bg`, `jobs`)       | ✅          | ✅   | ✅  | May be interactive-only |
|                       | Process substitution (`<(...)`, `>(...)`)       | ❌          | ✅   | ❌  | |
| **Arrays**            | Indexed arrays                                  | ❌          | ✅   | ✅  | `arr=(a b c)` |
|                       | Associative arrays                              | ❌          | ✅   | ❌  | `declare -A` |
| **Parameter Expansion** | `${var}` basic expansion                    | ✅          | ✅   | ❌  | |
|                       | `${var:-default}`, `${var:=default}`            | ✅          | ✅   | ❌  | |
|                       | `${#var}`, `${var#pattern}`                     | ✅          | ✅   | ❌  | |
|                       | `${!var}` indirect expansion                    | ❌          | ✅   | ❌  | |
|                       | `${var[@]}` / `${var[*]}` array expansion       | ❌          | ✅   | ❌  | |
| **Command Execution** | Pipelines (`|`)                                 | ✅          | ✅   | ❌  | |
|                       | Logical AND / OR (`&&`, `||`)                   | ✅          | ✅   | ❌  | |
|                       | Grouping (`( )`, `{ }`)                         | ✅          | ✅   | ❌  | |
|                       | Subshell (`( )`)                                | ✅          | ✅   | ❌  | |
|                       | Coprocesses (`coproc`)                          | ❌          | ✅   | ❌  | |
| **Builtins**          | `cd`, `echo`, `test`, `read`, `eval`, etc.      | ✅          | ✅   | ✅  | |
|                       | `shopt`, `declare`, `typeset`                   | ❌          | ✅   | ❌  | Bash-only |
|                       | `let`, `local`, `export`                        | ✅          | ✅   | ❌  | |
| **Debugging**         | `set -x`, `set -e`, `trap`                      | ✅          | ✅   | ❌  | |
|                       | `BASH_SOURCE`, `FUNCNAME` arrays                | ❌          | ✅   | ❌  | |
| **Miscellaneous**     | Brace expansion (`{1..5}`)                      | ❌          | ✅   | ❌  | |
|                       | Extended globbing (`extglob`)                   | ❌          | ✅   | ❌  | Requires `shopt` |
|                       | Bash version variables (`$BASH_VERSION`)        | ❌          | ✅   | ❌  | |
|                       | Source other scripts (`.` or `source`)          | ✅          | ✅   | ❌  | `source` is Bash synonym |

## Install as your shell

> ⚠️ Myst is still under development. Use it with caution in production environments.

Option 1:

```bash
cargo install flash
```

Option 2:

```bash
git clone https://github.com/raphamorim/flash.git
cd flash && cargo install --path .
```

Option 3:

```bash
git clone https://github.com/raphamorim/flash.git
cd flash
cargo build --release

# Linux
sudo cp target/release/flash /bin/

# MacOS/BSD
sudo cp target/release/flash /usr/local/bin/

# Done
flash
```

## Set as default

Optionally you can also set as default

```bash
# Add your flash path to:
vim /etc/shells

# Linux:
chsh -s /bin/flash

# MacOS/BSD:
chsh -s /usr/local/bin/flash
```

## 🔌 Embed in Your Rust Project

#### As an Interpreter

```rust
use flash::interpreter::Interpreter;
use std::io;

fn main() -> io::Result<()> {
    let mut interpreter = Interpreter::new();
    interpreter.run_interactive()?;
    Ok(())
}
```

Note that `run_interactive` will use flash default evaluator.

```rust
// Default interactive shell using DefaultEvaluator
pub fn run_interactive(&mut self) -> io::Result<()> {
    let default_evaluator = DefaultEvaluator;
    self.run_interactive_with_evaluator(default_evaluator)
}
```

You can actually create your own evaluator using Evaluator trait:

```rust
// Define the evaluation trait that users can implement
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
```

#### As a Lexer/Tokenizer

```rust
fn test_tokens(input: &str, expected_tokens: Vec<TokenKind>) {
    let mut lexer = Lexer::new(input);
    for expected in expected_tokens {
        let token = lexer.next_token();
        assert_eq!(
            token.kind, expected,
            "Expected {:?} but got {:?} for input: {}",
            expected, token.kind, input
        );
    }

    // Ensure we've consumed all tokens
    let final_token = lexer.next_token();
    assert_eq!(
        final_token.kind,
        TokenKind::EOF,
        "Expected EOF but got {:?}",
        final_token.kind
    );
}

#[test]
fn test_function_declaration() {
    let input = "function greet() { echo hello; }";
    let expected = vec![
        TokenKind::Function,
        TokenKind::Word("greet".to_string()),
        TokenKind::LParen,
        TokenKind::RParen,
        TokenKind::LBrace,
        TokenKind::Word("echo".to_string()),
        TokenKind::Word("hello".to_string()),
        TokenKind::Semicolon,
        TokenKind::RBrace,
    ];
    test_tokens(input, expected);
}
```

#### As a Parser

```rust
use flash::lexer::Lexer;
use flash::parser::Parser;

#[test]
fn test_simple_command() {
    let input = "echo hello world";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let result = parser.parse_script();

    match result {
        Node::List {
            statements,
            operators,
        } => {
            assert_eq!(statements.len(), 1);
            assert_eq!(operators.len(), 0);

            match &statements[0] {
                Node::Command {
                    name,
                    args,
                    redirects,
                } => {
                    assert_eq!(name, "echo");
                    assert_eq!(args, &["hello", "world"]);
                    assert_eq!(redirects.len(), 0);
                }
                _ => panic!("Expected Command node"),
            }
        }
        _ => panic!("Expected List node"),
    }
}
```

#### As Formatter

```rust
assert_eq!(
    Formatter::format_str("       # This is a comment"),
    "# This is a comment"
);
```

Or by receiving AST

```rust
let mut formatter = Formatter::new();
let node = Node::Comment(" This is a comment".to_string());

assert_eq!(formatter.format(&node), "# This is a comment");
```

## 📦 Crate Info

Add Myst to your Cargo.toml:

```toml
flash = "0.x"
```

## TODO

- [ ] Functions for parser and interop.
- [ ] Loops for parser and interop.
- [ ] Array index references.

## Resources

- https://www.gnu.org/software/bash/manual/bash.html
- https://www.shellcheck.net/
- https://stackblitz.com/edit/bash-ast?file=src%2Fapp%2Fapp.component.ts

## License

[GPL-3.0 License](LICENSE) © [Raphael Amorim](https://github.com/raphamorim/)