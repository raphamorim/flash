# mystsh

Mystical shell parser, formatter, and interpreter with Bash support.

This library provides a fast and extensible **parser**, **formatter**, and **interpreter** for POSIX-style shell scripts, written entirely in **Rust**. Inspired by [mvdan/sh](https://pkg.go.dev/mvdan.cc/sh/v3/syntax), but built from the ground up for performance, correctness, and hackability.

It understands real-world shell code, handles edge cases, and offers structured access to ASTs for tooling, analysis, or code transformation.

## Why?

Dude, I just want to learn.

## Install

```bash
git clone https://github.com/raphamorim/mystsh.git
cd mystsh
cargo build --release

# MacOS/BSD: Change /bin/ to /usr/local/bin/
sudo cp target/release/myst /bin/
myst
```