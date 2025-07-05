# Flash Shell

*A POSIX-compliant shell parser, formatter, and interpreter implemented in Rust.*

Flash is a high-performance, extensible toolkit for processing POSIX-style shell scripts. The system comprises three primary components: a lexical analyzer, a syntax parser, and an execution interpreter, all implemented from the ground up in Rust. Flash provides comprehensive support for real-world shell syntax and offers structured Abstract Syntax Tree (AST) access for static analysis, code transformation, and tooling development.

The project draws inspiration from [mvdan/sh](https://pkg.go.dev/mvdan.cc/sh/v3/syntax) while prioritizing performance optimization and architectural extensibility through modern systems programming practices.

**Development Status**: This project is currently under active development and should be considered experimental for production use cases.

## Table of Contents

- [Feature Coverage](#feature-coverage)
- [Shell Implementation](#shell-implementation)
- [Library Integration](#library-integration)

## Feature Coverage

The following table provides a comprehensive overview of POSIX Shell and Bash feature support within the Flash implementation. This matrix serves as both a development roadmap and compatibility reference.

**Legend:**
- **Fully Supported**: Complete implementation with full functionality
- **Parser Only**: Syntax recognition and AST generation without execution support
- **Not Supported**: Feature not currently implemented

| Category              | Functionality / Feature                         | POSIX Shell | Bash | Flash | Implementation Notes |
|-----------------------|--------------------------------------------------|-------------|------|-------|---------------------|
| **Basic Syntax**      | Variable assignment                             | Fully Supported          | Fully Supported   | Fully Supported  | `VAR=value` syntax |
|                       | Command substitution                            | Fully Supported          | Fully Supported   | Fully Supported  | Both `$(cmd)` and `` `cmd` `` forms |
|                       | Arithmetic substitution                         | Not Supported          | Fully Supported   | Fully Supported  | `$((expr))` evaluation |
|                       | Comments (`#`)                                  | Fully Supported          | Fully Supported   | Fully Supported  | Standard comment syntax |
|                       | Quoting (`'`, "", `\`)                          | Fully Supported          | Fully Supported   | Fully Supported  | All quoting mechanisms |
|                       | Globbing (`*`, `?`, `[...]`)                    | Fully Supported          | Fully Supported   | Fully Supported  | Pattern matching |
| **Control Structures**| `if` / `else` / `elif`                          | Fully Supported          | Fully Supported   | Fully Supported  | Conditional execution |
|                       | `case` / `esac`                                 | Fully Supported          | Fully Supported   | Fully Supported  | Pattern matching constructs |
|                       | `for` loops                                     | Fully Supported          | Fully Supported   | Fully Supported  | Iteration constructs |
|                       | `while`, `until` loops                          | Fully Supported          | Fully Supported   | Fully Supported  | Loop constructs |
|                       | `select` loop                                   | Not Supported          | Fully Supported   | Fully Supported  | Interactive selection |
|                       | `[[ ... ]]` test command                        | Not Supported          | Fully Supported   | Fully Supported  | Extended test expressions |
| **Functions**         | Function definition (`name() {}`)               | Fully Supported          | Fully Supported   | Fully Supported  | Standard function syntax |
|                       | `function` keyword                              | Not Supported          | Fully Supported   | Fully Supported  | Bash-specific syntax |
| **I/O Redirection**   | Output/input redirection (`>`, `<`, `>>`)       | Fully Supported          | Fully Supported   | Fully Supported  | Standard redirection |
|                       | Here documents (`<<`, `<<-`)                    | Fully Supported          | Fully Supported   | Parser Only  | Partial implementation |
|                       | Here strings (`<<<`)                            | Not Supported          | Fully Supported   | Parser Only  | Partial implementation |
|                       | File descriptor duplication (`>&`, `<&`)        | Fully Supported          | Fully Supported   | Parser Only  | Partial implementation |
| **Job Control**       | Background execution (`&`)                      | Fully Supported          | Fully Supported   | Fully Supported  | Process backgrounding |
|                       | Job control commands (`fg`, `bg`, `jobs`)       | Fully Supported          | Fully Supported   | Fully Supported  | Interactive mode only |
|                       | Process substitution (`<(...)`, `>(...)`)       | Not Supported          | Fully Supported   | Parser Only  | Basic `<(cmd)` support |
| **Arrays**            | Indexed arrays                                  | Not Supported          | Fully Supported   | Fully Supported  | `arr=(a b c)` syntax |
|                       | Associative arrays                              | Not Supported          | Fully Supported   | Not Supported  | `declare -A` requirement |
| **Parameter Expansion** | `${var}` basic expansion                      | Fully Supported          | Fully Supported   | Fully Supported  | Variable expansion framework |
|                       | `${var:-default}`, `${var:=default}`            | Fully Supported          | Fully Supported   | Fully Supported  | Default value expansion |
|                       | `${#var}`, `${var#pattern}`                     | Fully Supported          | Fully Supported   | Fully Supported  | Length and pattern operations |
|                       | `${!var}` indirect expansion                    | Not Supported          | Fully Supported   | Fully Supported  | Variable indirection |
|                       | `${var[@]}` / `${var[*]}` array expansion       | Not Supported          | Fully Supported   | Not Supported  | Array element expansion |
| **Command Execution** | Pipelines                                       | Fully Supported          | Fully Supported   | Fully Supported  | Command chaining |
|                       | Logical AND / OR (`&&`, `||`)                     | Fully Supported          | Fully Supported   | Fully Supported  | Conditional execution |
|                       | Grouping (`( )`, `{ }`)                         | Fully Supported          | Fully Supported   | Fully Supported  | Command grouping |
|                       | Subshell (`( )`)                                | Fully Supported          | Fully Supported   | Fully Supported  | Isolated execution context |
|                       | Coprocesses (`coproc`)                          | Not Supported          | Fully Supported   | Not Supported  | Bidirectional pipes |
| **Builtins**          | `cd`, `echo`, `test`, `read`, `eval`, etc.      | Fully Supported          | Fully Supported   | Fully Supported  | Core built-in commands |
|                       | `shopt`, `declare`, `typeset`                   | Not Supported          | Fully Supported   | Not Supported  | Bash-specific builtins |
|                       | `let`, `local`, `export`                        | Fully Supported          | Fully Supported   | Fully Supported  | Variable management |
| **Debugging**         | `set -x`, `set -e`, `trap`                      | Fully Supported          | Fully Supported   | Parser Only  | Partial debugging support |
|                       | `BASH_SOURCE`, `FUNCNAME` arrays                | Not Supported          | Fully Supported   | Not Supported  | Runtime introspection |
| **Miscellaneous**     | Brace expansion (`{1..5}`)                      | Not Supported          | Fully Supported   | Fully Supported  | Sequence generation |
|                       | Extended globbing (`extglob`)                   | Not Supported          | Fully Supported   | Not Supported  | Requires `shopt` configuration |
|                       | Version variables (`$BASH_VERSION`)        | Not Supported          | Fully Supported   | Fully Supported  | `$FLASH_VERSION` in Flash |
|                       | Script sourcing (`.` or `source`)          | Fully Supported          | Fully Supported   | Fully Supported  | External script inclusion |

## Shell Implementation

### Theoretical Foundation

A shell fundamentally operates as a macro processor that executes commands, where macro processing refers to the expansion of text and symbols into more complex expressions. The Unix shell paradigm encompasses dual functionality: serving as both a command interpreter and a programming language environment.

As a command interpreter, the shell provides the primary user interface to the comprehensive suite of Unix utilities and system commands. The programming language capabilities enable the composition and combination of these utilities into more sophisticated operations. Shell scripts, containing sequences of commands, achieve the same execution status as system binaries located in standard directories such as `/bin`, enabling users and organizations to establish customized automation environments.

Shell execution operates in two primary modes: interactive and non-interactive. Interactive mode processes user input from keyboard interfaces in real-time, while non-interactive mode executes command sequences from script files.

Flash maintains substantial compatibility with both POSIX shell (`sh`) and Bash specifications, implementing the core language features and execution semantics expected by existing shell scripts.

**Production Readiness**: Flash is currently in active development and should be evaluated carefully before deployment in production environments.

### Installation Methods

#### Method 1: Cargo Package Manager
```bash
cargo install flash
```

#### Method 2: Source Installation
```bash
git clone https://github.com/raphamorim/flash.git
cd flash && cargo install --path .
```

#### Method 3: Manual Binary Installation
```bash
git clone https://github.com/raphamorim/flash.git
cd flash
cargo build --release

# Linux systems
sudo cp target/release/flash /bin/

# macOS/BSD systems
sudo cp target/release/flash /usr/local/bin/

# Verify installation
flash
```

### System Integration

#### Default Shell Configuration

To configure Flash as the default system shell:

```bash
# Add Flash binary path to system shells registry
vim /etc/shells

# Linux systems
chsh -s /bin/flash

# macOS/BSD systems
chsh -s /usr/local/bin/flash
```

### Configuration Management

Flash implements a configuration system through the `.flashrc` initialization file located in the user's home directory. This file executes during shell startup, enabling environment customization and initialization script execution.

#### Prompt Customization

The shell prompt can be customized through the `PROMPT` environment variable within the `.flashrc` configuration file:

```bash
# Minimal prompt configuration
export PROMPT="flash> "

# Directory-aware prompt
export PROMPT='flash:$PWD$ '

# Full context prompt with user and hostname
export PROMPT='$USER@$HOSTNAME:$PWD$ '
```

The `PROMPT` variable supports full variable expansion, allowing integration of any available environment variables into the prompt display.

#### Configuration Example

```bash
# Prompt configuration
export PROMPT='flash:$PWD$ '

# Standard environment variables
export EDITOR=vim
export PAGER=less

# Future alias support (planned feature)
# alias ll="ls -la"
# alias grep="grep --color=auto"
```

---

## Library Integration

Flash provides comprehensive library functionality for integration into Rust applications, supporting multiple use cases including testing frameworks, shell script parsing, custom shell backend development, code formatting, and static analysis tooling.

### Interpreter Integration

The Flash interpreter can be embedded directly into Rust applications:

```rust
use flash::interpreter::Interpreter;
use std::io;

fn main() -> io::Result<()> {
    let mut interpreter = Interpreter::new();
    interpreter.run_interactive()?;
    Ok(())
}
```

The `run_interactive` method utilizes Flash's default evaluation engine:

```rust
// Default interactive shell implementation
pub fn run_interactive(&mut self) -> io::Result<()> {
    let default_evaluator = DefaultEvaluator;
    self.run_interactive_with_evaluator(default_evaluator)
}
```

### Custom Evaluation Engine

Flash supports custom evaluation logic through the `Evaluator` trait, enabling specialized shell behavior:

```rust
// Evaluation trait for custom implementations
pub trait Evaluator {
    fn evaluate(&mut self, node: &Node, interpreter: &mut Interpreter) -> Result<i32, io::Error>;
}

// Standard shell behavior implementation
pub struct DefaultEvaluator;

impl Evaluator for DefaultEvaluator {
    fn evaluate(&mut self, node: &Node, interpreter: &mut Interpreter) -> Result<i32, io::Error> {
        match node {
            Node::Command { name, args, redirects } => {
                self.evaluate_command(name, args, redirects, interpreter)
            }
            Node::Pipeline { commands } => {
                self.evaluate_pipeline(commands, interpreter)
            }
            Node::List { statements, operators } => {
                self.evaluate_list(statements, operators, interpreter)
            }
            Node::Assignment { name, value } => {
                self.evaluate_assignment(name, value, interpreter)
            }
            // Additional node types...
            _ => Err(io::Error::other("Unsupported node type")),
        }
    }
}
```

The `DefaultEvaluator` implements comprehensive shell semantics including built-in command handling, pipeline execution, variable assignment, and external command invocation with proper environment variable propagation and I/O redirection support.

### Lexical Analysis

Flash provides direct access to its lexical analyzer for token-level processing:

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

    // Verify complete token consumption
    let final_token = lexer.next_token();
    assert_eq!(final_token.kind, TokenKind::EOF);
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

### Syntax Analysis

The parser component transforms token streams into structured Abstract Syntax Trees:

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
        Node::List { statements, operators } => {
            assert_eq!(statements.len(), 1);
            assert_eq!(operators.len(), 0);

            match &statements[0] {
                Node::Command { name, args, redirects } => {
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

### Code Formatting

Flash includes a comprehensive formatter for shell script standardization:

```rust
// String-based formatting
assert_eq!(
    Formatter::format_str("       # This is a comment"),
    "# This is a comment"
);
```

```rust
// AST-based formatting
let mut formatter = Formatter::new();
let node = Node::Comment(" This is a comment".to_string());

assert_eq!(formatter.format(&node), "# This is a comment");
```

## References

- [GNU Bash Manual](https://www.gnu.org/software/bash/manual/bash.html)
- [ShellCheck Static Analysis Tool](https://www.shellcheck.net/)
- [Bash AST Visualization](https://stackblitz.com/edit/bash-ast?file=src%2Fapp%2Fapp.component.ts)

## License

This project is licensed under the [GPL-3.0 License](LICENSE). Copyright © [Raphael Amorim](https://github.com/raphamorim/).