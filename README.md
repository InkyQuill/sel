# sel — Select Slices from Text Files

Compact CLI utility for extracting fragments from text files by line numbers, ranges, positions (line:column), or regex patterns.

## Features

- Extract by line numbers: `sel 30-35 file.txt`
- Extract by ranges and lists: `sel 1,5,10-15,20 file.txt`
- Extract by positions with character context: `sel -n 10 23:260 file.txt`
- Regex search: `sel -e ERROR log.txt`
- Line context: `sel -c 3 42 file.txt`
- Streaming: works with very large files without loading into memory
- Color output: `sel --color=always -e ERROR log.txt`

## Examples

```bash
# Output lines 30-35
sel 30-35 file.txt

# Output lines 10, 15-20, and 22
sel 10,15-20,22 file.txt

# Show line 42 with 3 lines of context
sel -c 3 42 file.txt

# Show position line 23, column 260 with 10 chars of context
sel -n 10 23:260 file.txt

# Context + char context
sel -c 2 -n 5 15:30 file.txt

# Search for ERROR pattern
sel -e ERROR log.txt

# Search with context
sel -c 2 -e TODO source.rs

# Output entire file with line numbers (like cat -n)
sel file.txt
```

## Installation

```bash
cargo install sel
```

## Usage

```
sel [OPTIONS] <selector> <file>
sel [OPTIONS] -e <pattern> <file>...

Arguments:
  <selector>  Line number, range (M-N), list (N,M-N,P), position (L:C), or omit for all lines

Options:
  -c, --context <N>          Show N lines of context before and after matches
  -n, --char-context <N>     Show N characters of context around position
  -l, --no-line-numbers      Don't output line numbers
  -e, --regex <PATTERN>      Regular expression pattern
  -H, --with-filename        Always print filename prefix
      --color <WHEN>         Color output [auto, always, never] [default: auto]
  -h, --help                 Print help
  -V, --version              Print version
```

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Lint
cargo clippy
```

## License

MIT OR Apache-2.0

## Author

InkyQuill — <https://github.com/InkyQuill>
