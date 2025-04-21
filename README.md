# mystsh

Mystical shell parser, formatter, and interpreter with Bash support

This library provides a fast and extensible **parser**, **formatter**, and **interpreter** for POSIX-style shell scripts, written entirely in **Rust**. Inspired by [mvdan/sh](https://pkg.go.dev/mvdan.cc/sh/v3/syntax), but built from the ground up for performance, correctness, and hackability.

It understands real-world shell code, handles edge cases, and offers structured access to ASTs for tooling, analysis, or code transformation.

## ✨ Features

- ✅ Robust parser producing an abstract syntax tree (AST)
- ✅ Pretty-printer/formatter with customizable indentation
- ✅ Interpreter for executing shell scripts
- ✅ Interactive REPL mode
- ✅ Friendly, safe Rust API

---

## 🚀 Example: Parse, Format, Execute

```rust
use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Shell Parser, Formatter, and Interpreter Demo");

    demo_parser()?;
    demo_formatter()?;
    demo_interpreter()?;

    if env::args().any(|arg| arg == "--interactive") {
        let mut interpreter = Interpreter::new();
        interpreter.run_interactive()?;
    }

    Ok(())
}
```

## 🔍 Parsing

```bash
#!/bin/bash
echo "Hello, world!"
for i in $(seq 1 10); do
  echo "Count: $i"
done
```

The parser turns this into a typed AST structure:

```
List:
  Command: echo ["Hello, world!"]
  Operator: ;
  ForLoop: i in $(seq 1 10)
    Command: echo ["Count: $i"]
```

## 🎨 Formatting

Messy shell script?

```bash
if [ $x -eq 42 ]; then echo "The answer"; elif [ $x -lt 42 ]; then echo "Too low"; else echo "Too high"; fi
```

Formatted with consistent indentation:

```bash
if [ $x -eq 42 ]; then
  echo "The answer"
elif [ $x -lt 42 ]; then
  echo "Too low"
else
  echo "Too high"
fi
```

## 🧪 Interpreting

Run shell scripts programmatically (including variable handling, I/O, and exit codes):

```bash
MESSAGE="Hello from the interpreter"
echo $MESSAGE
```

Output:
Hello from the interpreter
Exit code: 0

💻 Interactive Mode

Launch an interactive shell with:

```bash
cargo run -- --interactive
$ echo hello
hello
$ exit
```

## 🔧 API

```rust
let ast = parse_script(script)?;
let formatted = format_script(script, "  ");
let result = Interpreter::new().execute(script)?;
```

## 🦀 Why Rust?

- Memory safety without GC
- Lightning-fast tooling
- Easy integration into other Rust projects
- Great for embedding in editors or dev tools

## 📦 Installation

```bash
cargo install mystsh
```

## 📚 License

MIT or Apache 2.0 — your choice.
