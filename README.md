# sel — Select Slices from Text Files

[![CI](https://github.com/InkyQuill/sel/actions/workflows/ci.yml/badge.svg)](https://github.com/InkyQuill/sel/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/sel-rs.svg)](https://crates.io/crates/sel-rs)
[![docs.rs](https://docs.rs/sel-rs/badge.svg)](https://docs.rs/sel-rs)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

**0.2.0** — compact CLI (and Rust library) for extracting fragments from
text by line numbers, ranges, positions (`line:column`), or regex. Streams
input so it scales to large files; supports stdin, file output, and colored
output with context.

## Documentation

| Resource | Description |
|----------|-------------|
| [docs/README.md](docs/README.md) | Index of all project docs |
| [docs/USAGE.md](docs/USAGE.md) | Full CLI reference |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Pipeline and module design |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Build, test, PR workflow |
| [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) | Community Code of Conduct |
| [CHANGELOG.md](CHANGELOG.md) | Release notes |
| [docs.rs/sel-rs](https://docs.rs/sel-rs) | Rust API reference |

## Features

- **Line selectors**: single (`42`), ranges (`10-20`), lists (`1,5,10-15,22`).
- **Positional selectors**: `line:column` with per-character context.
- **Regex search** (`-e PATTERN`) with optional invert (`-v`).
- **Line context** (`-c N`): N lines around each match, windows merged on overlap.
- **Streaming**: single-pass `BufReader` pipeline; memory independent of file size.
- **Stdin**: no file arg or `-` reads stdin (positional selectors need a seekable file).
- **Output to file** (`-o PATH`), never clobbers unless `--force`.
- **Color** (`--color auto|always|never`) with ANSI highlighting.

## Installation

```bash
# From crates.io (the crate is published as `sel-rs`; the binary is still `sel`)
cargo install sel-rs

# From source
git clone https://github.com/InkyQuill/sel.git
cd sel && cargo install --path .
```

Pre-built binaries for Linux, macOS, and Windows are published on the
[Releases](https://github.com/InkyQuill/sel/releases) page via
[`cargo-dist`](https://opensource.axo.dev/cargo-dist/).

### Claude Code skill (optional)

Teach Claude Code to prefer `sel` over `sed -n '…p'` for line extraction:

```bash
# User-level
curl -fsSL https://raw.githubusercontent.com/InkyQuill/sel/main/contrib/claude-skill/install.sh | sh

# Project-level (installs into ./.claude/skills/)
curl -fsSL https://raw.githubusercontent.com/InkyQuill/sel/main/contrib/claude-skill/install.sh | sh -s -- --project
```

See [`contrib/claude-skill/`](contrib/claude-skill/) for details and
uninstall instructions.

## Quick examples

```bash
# Lines 30–35
sel 30-35 file.txt

# A mix of singles and ranges
sel 1,5,10-15,22 file.txt

# Line 42 with 3 lines of context on each side
sel -c 3 42 file.txt

# Position (line 23, column 260) with 10 chars of context
sel -n 10 23:260 file.txt

# Regex search with context
sel -c 2 -e TODO src/main.rs

# Invert-match: lines that do NOT match
sel -v -e '^\s*#' config.ini

# Multiple files (filename prefix auto-enabled)
sel -e ERROR logs/*.log

# Stdin
cat huge.log | sel -e 'panic!'
sel 10-20 -            # explicit stdin sentinel

# Write to a file, refuse to overwrite
sel 1-100 big.txt -o head.txt
sel 1-100 big.txt -o head.txt --force    # overwrite

# Whole file with line numbers (like `cat -n`)
sel file.txt
```

## Usage (summary)

Run `sel --help` for the full option list. A detailed reference lives in
[`docs/USAGE.md`](docs/USAGE.md).

```
sel [OPTIONS] [SELECTOR_OR_FILE]...
sel [OPTIONS] -e <PATTERN> [FILE]...

Options:
  -c, --context <N>          Lines of context before/after matches
  -n, --char-context <N>     Character context (positions / regex)
  -l, --no-line-numbers      Suppress line numbers
  -e, --regex <PATTERN>      Regular expression pattern
  -v, --invert-match         With -e: emit non-matching lines
  -H, --with-filename        Always print filename prefix
      --color <WHEN>         auto | always | never (default: auto)
  -o, --output <FILE>        Write output to FILE (`-` = stdout)
      --force                With -o, overwrite an existing file
  -h, --help                 Print help
  -V, --version              Print version
```

## Development

Requires **Rust 1.92+** (edition 2024), as set in `Cargo.toml`.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo bench                    # Criterion: benches/large_file.rs
```

Contributions welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.

## Author

Pavel Obruchnikov a.k.a InkyQuill — <https://github.com/InkyQuill>
