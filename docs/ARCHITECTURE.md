# Architecture

**sel** is a Rust crate that exposes both a **`sel` binary** and a **library**
with the same streaming pipeline. This document summarizes how pieces fit
together; public types are documented in rustdoc on [docs.rs/sel](https://docs.rs/sel).

## High-level data flow

Every run follows one path:

```text
Source → Matcher → Expander → Formatter → Sink
```

1. **Source** — `BufReader`-style line iteration (`FileSource`, `StdinSource`).
2. **Matcher** — Per line, decides if/how the line matches: all lines, explicit line specs, byte/column positions, or regex.
3. **Expander** — Turns raw matches into **emits**: optional line context
   (`-c`), deferred until boundaries are known; `drain` flushes at EOF.
4. **Formatter** — Plain text, fragments with char context (`-n`), ANSI colors.
5. **Sink** — `StdoutSink` or `FileSink` (`-o`, respects `--force`).

The driver is a single function:

```rust
// crate::pipeline
pub fn run<K: SourceKind>(mut app: App<K>) -> Result<()>
```

It loops: read line → `match_line` → `expander.push` (which invokes the
formatter/sink callback) → after EOF, `expander.drain` → `sink.finish`.

## Typestate builder (`app.rs`)

`App<K>` is built through stages `Stage1 → … → Stage5` so **invalid
combinations are unrepresentable**:

- **`Seek` vs `NonSeek`** — `SourceKind` marks whether the source supports
  seeking. **Positional matchers** (`PositionMatcher`) are only constructed on
  `Seek` sources. Stdin forces `NonSeek`, so `line:column` selectors cannot be
  wired to stdin at compile time; the CLI mirrors this with a runtime check and
  error message.
- Stages attach `Source`, `Matcher`, `Expander`, `Formatter`, `Sink` in order.

## Modules (crate layout)

| Path | Role |
|------|------|
| `cli.rs` | `clap` definitions; resolves selector vs files; builds `App` |
| `app.rs` | `App`, stages, `SourceKind` / `Seekable` |
| `pipeline.rs` | `run()` loop |
| `selector.rs` | Parse and normalize `Selector`, `LineSpec`, `Position` |
| `context.rs` | `Expander`, `NoContext`, `LineContext` (merged windows) |
| `matcher/` | `Matcher` trait; line, position, regex, “all” matchers |
| `source/` | `Source` trait; file vs stdin |
| `format/` | `Formatter`; plain vs fragment; ANSI helpers |
| `sink/` | `Sink`; stdout vs file |
| `types.rs` | `Line`, `MatchInfo`, `Role`, `Emit` |
| `error.rs` | `SelError`, `Result` |

## Testing strategy

- **Integration tests** in `tests/` exercise behaviour end-to-end (selectors,
  regex, context, stdin, multiple files, output file, etc.).
- **Unit tests** live beside implementation (`#[cfg(test)]`).
- **Benchmarks** in `benches/large_file.rs` (Criterion) stress the streaming path.

## CI

GitHub Actions runs `fmt`, `clippy` (`-D warnings`), and `cargo test` on
Ubuntu, macOS, and Windows (see `.github/workflows/ci.yml`).

## Changing behaviour safely

- New CLI flags: extend `cli.rs`, thread options into builder or formatters,
  update `tests/` and `docs/USAGE.md`.
- New match kinds: implement `Matcher`, plug into stage2 builder paths.
- New output shapes: implement `Formatter` or extend `FormatOpts`.
