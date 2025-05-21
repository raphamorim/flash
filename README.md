# myst (work in progress)

*A mystical shell parser, formatter, and interpreter written in Rust.*

Myst is a fast, extensible, and hackable toolkit for working with POSIX-style shell scripts. It includes a parser, formatter, and interpreter built from scratch in Rust. Myst understands real-world shell syntax and provides structured AST access for static analysis, tooling, and transformation.

> Inspired by [mvdan/sh](https://pkg.go.dev/mvdan.cc/sh/v3/syntax), but engineered from the ground up with performance and extensibility in mind.

## Motivation

Myst was created to serve two main purposes: as a learning project to better understand shell parsing and syntax, and as a tool for testing and embedding within the [Rio terminal emulator](https://github.com/raphamorim/rio/), a GPU-accelerated terminal written in Rust.

## Feature Coverage

This table outlines the supported features of POSIX Shell and Bash. Use it to track what your **Myst** parser and interpreter implementation in Rust supports.

Legends:

- ✅ fully supported.
- ⚠️ only supported in parser and formatter.
- ❌ not supported.

| Category              | Functionality / Feature                         | POSIX Shell | Bash | Myst | Notes |
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
cargo install mystsh
```

Option 2:

```bash
git clone https://github.com/raphamorim/myst.git
cd myst && cargo install --path .
```

Option 3:

```bash
git clone https://github.com/raphamorim/myst.git
cd myst
cargo build --release

# Linux
sudo cp target/release/myst /bin/

# MacOS/BSD
sudo cp target/release/myst /usr/local/bin/

# Done
myst
```

## Set as default

Optionally you can also set as default

```bash
# Add your myst path to:
vim /etc/shells

# Linux:
chsh -s /bin/myst

# MacOS/BSD:
chsh -s /usr/local/bin/myst
```

## 🔌 Embed in Your Rust Project

#### As an Interpreter

```rust
use mystsh::interpreter::Interpreter;
use std::io;

fn main() -> io::Result<()> {
    let mut interpreter = Interpreter::new();
    interpreter.run_interactive()?;
    Ok(())
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
use mystsh::lexer::Lexer;
use mystsh::parser::Parser;

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
mystsh = "0.x"
```

## TODO

- [ ] Remove interop custom functions from `run_interop` and allow to receive as parameter. It will split the current code there to `bin.rs` file.
- [ ] Functions for parser and interop.
- [ ] Loops for parser and interop.
- [ ] Array index references.

## Resources

- https://www.gnu.org/software/bash/manual/bash.html
- https://www.shellcheck.net/
- https://stackblitz.com/edit/bash-ast?file=src%2Fapp%2Fapp.component.ts

## License

[GPL-3.0 License](LICENSE) © [Raphael Amorim](https://github.com/raphamorim/)