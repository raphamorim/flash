# Flash WebAssembly Parser Demo

This demo compiles the Flash shell parser to WebAssembly, allowing you to parse shell code directly in the browser.

**🔗 [Flash on GitHub](https://github.com/raphamorim/flash)**

## Features

- **Live Parsing**: Input shell code on the left, see the parsed AST on the right
- **Real-time Updates**: Parse code with Ctrl+Enter or the Parse button
- **Example Code**: Click on examples to quickly test different shell constructs
- **Error Handling**: Clear error messages for invalid syntax

## Building and Running

### Prerequisites

- Rust toolchain
- `wasm-pack` (will be installed automatically by Makefile)
- `cargo-server` (will be installed automatically by Makefile)
- Git LFS (for committing WebAssembly files)

### Build and Serve

From the project root directory:

```bash
# Build and serve the demo
make wasm-demo-serve

# Or just build without serving
make wasm-demo-build

# Build and prepare files for git commit
make wasm-demo-commit

# Clean build artifacts
make wasm-demo-clean
```

The demo will be available at `http://localhost:8000`

### Git LFS Setup

WebAssembly files (`.wasm`) are tracked using Git LFS to avoid bloating the repository:

- `.wasm` files are automatically tracked by Git LFS
- The `docs/pkg/` directory can be committed to the repository
- Use `make wasm-demo-commit` to build and stage files for commit

### Manual Serving

If you prefer to serve manually after building:

```bash
# Build first
make wasm-demo-build

# Then serve with cargo-server
cd docs
cargo server --port 8000

# Or use other methods
python3 -m http.server 8000
# or
npx serve . -p 8000
```

## Supported Shell Constructs

The demo can parse various shell constructs including:

- Simple commands: `ls -la`
- Pipelines: `ls | grep test`
- Redirections: `echo "hello" > file.txt`
- Variable assignments: `VAR=value`
- Command substitution: `echo $(date)`
- Arithmetic expansion: `echo $((2 + 2))`
- Conditional statements: `if [ condition ]; then ...; fi`
- Loops: `for i in {1..10}; do ...; done`
- Functions: `function name() { ...; }`
- And more!

## Architecture

- **Rust Library**: The core Flash parser compiled to WebAssembly
- **JavaScript Interface**: Wasm-bindgen provides the JS/WASM bridge
- **Web Interface**: Clean two-panel layout for input and output

The WebAssembly module exposes a `parse_shell_code` function that takes shell code as input and returns a structured representation of the parsed AST.