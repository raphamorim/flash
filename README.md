# myst (work in progress)

*A mystical shell parser, formatter, and interpreter written in Rust.*

Myst is a fast, extensible, and hackable toolkit for working with POSIX-style shell scripts. It includes a parser, formatter, and interpreter built from scratch in Rust. Myst understands real-world shell syntax and provides structured AST access for static analysis, tooling, and transformation.

> Inspired by [mvdan/sh](https://pkg.go.dev/mvdan.cc/sh/v3/syntax), but engineered from the ground up with performance and extensibility in mind.

## Motivation

Myst was created to serve two main purposes: as a learning project to better understand shell parsing and syntax, and as a tool for testing and embedding within the [Rio terminal emulator](https://github.com/raphamorim/rio/), a GPU-accelerated terminal written in Rust.

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

# MacOS/BSD: Change /bin/ to /usr/local/bin/
sudo cp target/release/myst /bin/
myst
```

#### Interpreter readline support

✅ - Kill Ring: Text deleted with Ctrl+K, Ctrl+U, or Ctrl+W is saved to a kill ring, allowing it to be yanked (pasted) with Ctrl+Y.
✅ - Ability to transpose characters with Ctrl+T.
✅ - Incremental History Search: Pressing Ctrl+R initiates a search through command history. Pressing Ctrl+R again searches for the next occurrence of the same pattern.
✅ - Bidirectional character and word movement keys.
✅ - Ctrl+A: Move cursor to the beginning of the line.
✅ - Ctrl+E: Move cursor to the end of the line.
✅ - Ctrl+B: Move backward one character (same as left arrow).
✅ - Ctrl+F: Move forward one character (same as right arrow).
✅ - Ctrl+K: Cut text from cursor to end of line (kill).
✅ - Ctrl+U: Cut text from beginning of line to cursor (kill).
✅ - Ctrl+Y: Paste (yank) previously killed text.
✅ - Ctrl+P: Previous history entry (same as up arrow).
✅ - Ctrl+N: Next history entry (same as down arrow).
✅ - Ctrl+T: Transpose (swap) characters at cursor.
✅ - Ctrl+D: Delete character under cursor (or exit if line is empty).
✅ - Ctrl+R: Reverse incremental search through history.
✅ - Ctrl+W: Delete word backward.
✅ - Ctrl+L: Clear screen and redraw prompt.
✅ - Ctrl+C: Cancel/interrupt.
✅ - Arrow keys for navigation and history.

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

## Myst Feature Coverage

This table outlines the supported features of POSIX Shell and Bash. Use it to track what your **Myst** parser and interpreter implementation in Rust supports.

Legends:

- ✅ fully supported.
- ⚠️ only supported in parser and formatter.
- ❌ not supported.

| Category              | Functionality / Feature                         | POSIX Shell | Bash | Myst | Notes |
|-----------------------|--------------------------------------------------|-------------|------|------|-------|
| **Basic Syntax**      | Variable assignment                             | ✅          | ✅   | ✅  | `VAR=value` |
|                       | Command substitution                            | ✅          | ✅   | [ ]  | `$(cmd)` and `` `cmd` `` |
|                       | Arithmetic substitution                         | ❌          | ✅   | [ ]  | `$((expr))` |
|                       | Comments (`#`)                                  | ✅          | ✅   | ✅  | |
|                       | Quoting (`'`, "", `\`)                          | ✅          | ✅   | [ ]  | |
|                       | Globbing (`*`, `?`, `[...]`)                    | ✅          | ✅   | [ ]  | |
| **Control Structures**| `if` / `else` / `elif`                          | ✅          | ✅   | [ ]  | |
|                       | `case` / `esac`                                 | ✅          | ✅   | [ ]  | |
|                       | `for` loops                                     | ✅          | ✅   | [ ]  | |
|                       | `while`, `until` loops                          | ✅          | ✅   | [ ]  | |
|                       | `select` loop                                   | ❌          | ✅   | [ ]  | |
|                       | `[[ ... ]]` test command                        | ❌          | ✅   | [ ]  | Extended test |
| **Functions**         | Function definition (`name() {}`)               | ✅          | ✅   | [ ]  | |
|                       | `function` keyword                              | ❌          | ✅   | [ ]  | Bash-specific |
| **I/O Redirection**   | Output/input redirection (`>`, `<`, `>>`)       | ✅          | ✅   | [ ]  | |
|                       | Here documents (`<<`, `<<-`)                    | ✅          | ✅   | [ ]  | |
|                       | Here strings (`<<<`)                            | ❌          | ✅   | [ ]  | |
|                       | File descriptor duplication (`>&`, `<&`)        | ✅          | ✅   | [ ]  | |
| **Job Control**       | Background execution (`&`)                      | ✅          | ✅   | [ ]  | |
|                       | Job control commands (`fg`, `bg`, `jobs`)       | ✅          | ✅   | [ ]  | May be interactive-only |
|                       | Process substitution (`<(...)`, `>(...)`)       | ❌          | ✅   | [ ]  | |
| **Arrays**            | Indexed arrays                                  | ❌          | ✅   | [ ]  | `arr=(a b c)` |
|                       | Associative arrays                              | ❌          | ✅   | [ ]  | `declare -A` |
| **Parameter Expansion** | `${var}` basic expansion                    | ✅          | ✅   | [ ]  | |
|                       | `${var:-default}`, `${var:=default}`            | ✅          | ✅   | [ ]  | |
|                       | `${#var}`, `${var#pattern}`                     | ✅          | ✅   | [ ]  | |
|                       | `${!var}` indirect expansion                    | ❌          | ✅   | [ ]  | |
|                       | `${var[@]}` / `${var[*]}` array expansion       | ❌          | ✅   | [ ]  | |
| **Command Execution** | Pipelines (`|`)                                 | ✅          | ✅   | [ ]  | |
|                       | Logical AND / OR (`&&`, `||`)                   | ✅          | ✅   | [ ]  | |
|                       | Grouping (`( )`, `{ }`)                         | ✅          | ✅   | [ ]  | |
|                       | Subshell (`( )`)                                | ✅          | ✅   | [ ]  | |
|                       | Coprocesses (`coproc`)                          | ❌          | ✅   | [ ]  | |
| **Builtins**          | `cd`, `echo`, `test`, `read`, `eval`, etc.      | ✅          | ✅   | [ ]  | |
|                       | `shopt`, `declare`, `typeset`                   | ❌          | ✅   | [ ]  | Bash-only |
|                       | `let`, `local`, `export`                        | ✅          | ✅   | [ ]  | |
| **Debugging**         | `set -x`, `set -e`, `trap`                      | ✅          | ✅   | [ ]  | |
|                       | `BASH_SOURCE`, `FUNCNAME` arrays                | ❌          | ✅   | [ ]  | |
| **Miscellaneous**     | Brace expansion (`{1..5}`)                      | ❌          | ✅   | [ ]  | |
|                       | Extended globbing (`extglob`)                   | ❌          | ✅   | [ ]  | Requires `shopt` |
|                       | Bash version variables (`$BASH_VERSION`)        | ❌          | ✅   | [ ]  | |
|                       | Source other scripts (`.` or `source`)          | ✅          | ✅   | [ ]  | `source` is Bash synonym |


## 📦 Crate Info

Add Myst to your Cargo.toml:

```toml
mystsh = "0.x"
```

## TODO

- [ ] If for parser and interop.
- [ ] Functions for parser and interop.
- [ ] Loops for parser and interop.
- [ ] Array for parser and interop.

## License

[GPL-3.0 License](LICENSE) © [Raphael Amorim](https://github.com/raphamorim/)