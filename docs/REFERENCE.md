# `sel` — Definitive Technical Reference

> A comprehensive reference to the design, internals, and library API of
> **`sel`** (Select Slices from Text Files). This document complements, not
> replaces, the short user-facing docs:
> [`README.md`](../README.md), [`USAGE.md`](USAGE.md),
> [`ARCHITECTURE.md`](ARCHITECTURE.md), and the rustdoc on
> [docs.rs/sel-rs](https://docs.rs/sel-rs).

- **Package**: `sel`
- **Version documented**: 0.2.0
- **Edition**: Rust 2024 (MSRV 1.92)
- **License**: MIT OR Apache-2.0

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [System Overview](#2-system-overview)
3. [Design Decisions](#3-design-decisions)
4. [The Pipeline in Detail](#4-the-pipeline-in-detail)
5. [Core Data Types](#5-core-data-types)
6. [Module Reference](#6-module-reference)
7. [Selector Grammar](#7-selector-grammar)
8. [Matchers](#8-matchers)
9. [Context Expansion](#9-context-expansion)
10. [Formatters & ANSI](#10-formatters--ansi)
11. [Sources & Sinks](#11-sources--sinks)
12. [Error Model](#12-error-model)
13. [CLI Wiring](#13-cli-wiring)
14. [Library API & Embedding](#14-library-api--embedding)
15. [Extension Points](#15-extension-points)
16. [Performance Characteristics](#16-performance-characteristics)
17. [Testing Strategy](#17-testing-strategy)
18. [Build, Release & CI](#18-build-release--ci)
19. [Troubleshooting & Pitfalls](#19-troubleshooting--pitfalls)
20. [Glossary](#20-glossary)
21. [Appendix A — File Index](#appendix-a--file-index)
22. [Appendix B — Reading Paths](#appendix-b--reading-paths)

---

## 1. Executive Summary

`sel` is a small (~2 kLoC) Rust crate that provides a CLI for extracting
ranges, positions, or regex matches from text streams, plus a library that
exposes the same pipeline for embedding in other Rust tools.

It is designed around four invariants:

1. **Streaming** — one `BufReader` pass over the input; memory usage is
   independent of file size. A 100 GB log and a 10 kB config travel the same
   path.
2. **One pipeline** — every invocation (`sel 10-20`, `sel -e ERROR`,
   `sel file.txt`) produces the same five-stage pipeline:
   `Source → Matcher → Expander → Formatter → Sink`.
3. **Typestate builder** — invalid pipelines (positional selector on stdin)
   fail at **compile time** inside the crate, and with a helpful message at
   **parse time** from the CLI.
4. **No heavy dependencies** — three runtime crates (`clap`, `regex`,
   `thiserror`); no `anyhow`, no `termcolor`, no `is-terminal`. Color is a
   handful of ANSI codes in [`format/ansi.rs`](../src/format/ansi.rs).

The rest of this document walks through those ideas and the code that
realizes them.

---

## 2. System Overview

### 2.1 What `sel` does

Given a text source (a file, stdin, or multiple files), `sel` emits a subset
of the lines, optionally with surrounding context, optionally with
highlighting, to a sink (stdout or a file).

The subset is one of:

| Mode | Triggered by | Example |
|---|---|---|
| **All** | bare file, no selector | `sel file.txt` |
| **Line numbers** | digits, commas, hyphens | `sel 1,5,10-20 file.txt` |
| **Positions** | `line:column` | `sel -n 10 23:260 file.txt` |
| **Regex** | `-e PATTERN` | `sel -e TODO src/*.rs` |
| **Invert regex** | `-v -e PATTERN` | `sel -v -e '^\s*#' cfg.ini` |

### 2.2 Crate topology

```
sel (crate)
├── binary `sel` (src/main.rs)         <─┐
│                                        │  both go through
└── library `sel` (src/lib.rs)           │  pipeline::run
    ├── cli.rs   (argv  → App)        <──┘
    ├── app.rs   (typed builder)
    ├── pipeline.rs (driver)
    ├── source/  (file, stdin)
    ├── matcher/ (all, lines, position, regex)
    ├── context.rs (no-context, line-context)
    ├── format/  (plain, fragment, ansi)
    ├── sink/    (stdout, file)
    ├── selector.rs (parser)
    ├── types.rs    (Line, MatchInfo, Emit, Role)
    └── error.rs    (SelError)
```

The CLI is a very thin wrapper around the library: `main.rs` is **30 lines**
that iterate over file arguments, ask `Cli` to build an `App` for each, and
hand it to `pipeline::run`.

### 2.3 Data flow at a glance

```text
  argv
   │
   ▼
  Cli::parse ──► Cli::validate ──► for each file ──► Cli::into_app_for_*
                                                         │
                                                         ▼
                                                  App<K: SourceKind>
                                                         │
                                                         ▼
  ┌───────────────────────── pipeline::run ──────────────────────────┐
  │                                                                  │
  │    Source ──► Matcher ──► Expander ──► Formatter ──► Sink        │
  │   (Line)    (MatchInfo)  (EmitOwned)    (bytes)     (flush)      │
  │                                                                  │
  └──────────────────────────────────────────────────────────────────┘
```

Each stage is a trait object. Stages communicate through a tiny value
vocabulary: [`Line`](../src/types.rs), [`MatchInfo`](../src/types.rs),
[`EmitOwned`](../src/context.rs), and [`Emit`](../src/types.rs).

---

## 3. Design Decisions

This section documents the **why**. The **what** is in subsequent chapters.

### 3.1 Why a single pipeline, not a dispatch tree?

Earlier revisions of `sel` had one control-flow path per mode (line mode,
position mode, regex mode). Adding a new flag or context rule meant editing
all paths. The 0.2 refactor collapsed them into one loop:

```rust
// src/pipeline.rs
while let Some(line) = app.source.next_line()? {
    let info = app.matcher.match_line(&line);
    app.expander.push(line, info, &mut |emit| {
        formatter.write(sink.as_mut(), &emit.borrow())
    });
}
app.expander.drain(...);
sink.finish()?;
```

One loop, one place to change iteration semantics, one place to reason about
ownership. New features are almost always a swap of one stage.

See [`src/pipeline.rs:8`](../src/pipeline.rs) for the driver.

### 3.2 Why a typestate builder?

A positional selector (`line:column`) is meaningful only on a **seekable**
source; asking stdin for "column 260 on line 23" is nonsense because a pipe
cannot be addressed. The crate could detect this at runtime, but it is also
trivially representable in the type system:

- `Seek` and `NonSeek` are marker types implementing `SourceKind`.
- `Seekable: SourceKind` is a further marker that only `Seek` implements.
- `Stage2::with_position_matcher` is defined **only for `Stage2<Seek>`**.

Concrete effect: the Rust compiler refuses to compile
`StdinSource → PositionMatcher`. The CLI mirrors this at runtime with the
`SelError::PositionalWithStdin` variant for users who hit it.

See [`src/app.rs:11`](../src/app.rs) for the markers and
[`src/app.rs:66`](../src/app.rs) for the seek-restricted stage.

### 3.3 Why trait objects (`Box<dyn Trait>`)?

Stages are swappable at runtime based on flags. Static dispatch would
explode into a combinatorial number of monomorphizations (matcher × expander
× formatter × sink). The streaming pipeline is I/O-bound, so vtable overhead
is invisible. Trait objects give us:

- ~30 lines of pipeline driver code instead of generic gymnastics,
- simple CLI wiring in `cli.rs`,
- easy third-party extensibility (implement a trait, pass a `Box<dyn …>`).

### 3.4 Why strip dependencies?

`anyhow`, `termcolor`, and `is-terminal` were dropped between 0.1 and 0.2.

- **`anyhow`** — error domain is small and well-enumerated; `thiserror`
  gives structured errors that tests can match on.
- **`termcolor`** — `sel` emits three ANSI codes (green, inverse, reset).
  A 14-line module (`format/ansi.rs`) replaces the dependency.
- **`is-terminal`** — stabilised in `std::io::IsTerminal`; no need for the
  crate.

The result: faster builds, less surface area to vet, zero behavioural loss.

### 3.5 Why normalize selectors?

Users can (and do) write `sel 1,2,3,10-20,15,16 file.txt`. Left raw, each
line would be checked against six specs. The `Selector::normalize()` step
merges into `[1-3, 10-20]`:

- Two ranges instead of six specs → fewer comparisons per line.
- Predictable behaviour when specs overlap.
- Stable ordering for tests and snapshot comparisons.

`LineMatcher::from_selector` always runs `normalize()` first
([`src/matcher/lines.rs:15`](../src/matcher/lines.rs)).

### 3.6 Why bytes, not `&str`, for lines?

Input files are not guaranteed to be valid UTF-8. `Source` yields
`Line { no, bytes: Vec<u8> }`. The regex crate's `bytes::Regex` matches
against `&[u8]` directly; formatting uses `String::from_utf8_lossy` to
surface malformed UTF-8 as U+FFFD without crashing.

This matters for grepping binary-ish logs, Windows UTF-16 accidentally
piped in, or files with shift-JIS regions.

### 3.7 Why `Role` (Target vs Context)?

The formatter needs to know whether to prepend `"> "` (green target marker)
and whether the `match_info.spans` are meaningful (context lines inherit
the context-window, not the hit's spans). `Role::Target | Role::Context`
keeps that information on the `Emit` itself so formatters stay stateless.

---

## 4. The Pipeline in Detail

### 4.1 Stage contracts

| Stage | Trait | Input | Output |
|---|---|---|---|
| Source | `Source` | — | `Option<Line>` |
| Matcher | `Matcher` | `&Line` | `MatchInfo` |
| Expander | `Expander` | `Line, MatchInfo` | zero or more `EmitOwned` |
| Formatter | `Formatter` | `&Emit` | bytes to sink |
| Sink | `Sink: Write` | bytes | flushed I/O |

Ownership moves forward down the stages:

- The source owns the I/O handle.
- `Line::bytes` is a fresh `Vec<u8>` per call (no shared buffer — a
  deliberate simplification that keeps the expander's buffering trivial).
- The expander owns the `Line` until it hands it to the formatter inside
  an `EmitOwned`.
- The formatter borrows the emit and writes bytes to a `Sink`.

### 4.2 Walkthrough: `sel -c 2 -e ERROR file.log`

1. **CLI**: `Cli::parse` captures `regex = "ERROR"`, `context = Some(2)`;
   `get_files()` returns `["file.log"]`.
2. **Build**: `into_app_for_file` returns
   `App<Seek> { source: FileSource, matcher: RegexMatcher, expander: LineContext(2), formatter: PlainFormatter, sink: StdoutSink }`.
3. **Run**: `pipeline::run` loops. For each line:
   - `FileSource::next_line` yields `Line { no, bytes }`.
   - `RegexMatcher::match_line` returns `MatchInfo { hit, spans, col: None }`.
   - `LineContext::push` either:
     - Buffers the line (miss, no active trailing window).
     - Flushes the "before" buffer and emits the hit + starts a trailing
       window of 2.
     - Emits the line as context if we're within a trailing window.
4. **Output**: `PlainFormatter::write` prints `[prefix]content\n`, with
   green `"> "` on target lines because `target_marker` is on.
5. **Finish**: `sink.finish()` flushes the `BufWriter`.

### 4.3 The `Emit` / `EmitOwned` split

`EmitOwned` carries a `Line` by value through the expander's callback so
the expander can hold onto (or discard) buffered lines. The formatter
receives a borrowed `Emit<'_>` because it only needs to read. Conversion is
cheap: `EmitOwned::borrow` constructs the borrowed view in place
([`src/context.rs:27`](../src/context.rs)).

### 4.4 Back-pressure & streaming

`sel` has no explicit back-pressure; the `BufWriter` on the sink and the
OS write() call provide flow control naturally. Memory high-water mark is:

- `Line::bytes` currently in flight (one line).
- `before` ring buffer of at most `n` lines in `LineContext` where `n` is
  the `-c` value.
- `spans: Vec<Range<usize>>` for regex hits on the current line.

For a 100 GB file with `-c 3` and no ridiculous lines, peak memory stays
in the tens of kilobytes.

---

## 5. Core Data Types

### 5.1 `Line`

```rust
// src/types.rs:7
pub struct Line {
    pub no: u64,        // 1-indexed line number
    pub bytes: Vec<u8>, // line contents, newline stripped (both \n and \r\n)
}
```

Produced by sources, consumed by matchers (by reference), and carried
through the expander until emitted. Newline stripping is the source's
responsibility; both `\n` and `\r\n` are handled
([`src/source/file.rs:51`](../src/source/file.rs)).

### 5.2 `MatchInfo`

```rust
// src/types.rs:24
pub struct MatchInfo {
    pub hit: bool,
    pub spans: Vec<Range<usize>>, // byte ranges for regex highlighting
    pub col: Option<usize>,       // 1-indexed target column (positions)
}
```

`MatchInfo::default()` is a miss. Populated by each matcher according to
its nature: regex sets `spans`, position sets `col`, line matchers set
neither.

### 5.3 `Role`

```rust
// src/types.rs:36
pub enum Role { Target, Context }
```

Whether this emit is the line that matched (`Target`) or a context
neighbour (`Context`). Formatters consult this to decide on markers and
span highlighting.

### 5.4 `Emit<'a>` and `EmitOwned`

```rust
// src/types.rs:45
pub struct Emit<'a> {
    pub line: &'a Line,
    pub role: Role,
    pub match_info: &'a MatchInfo,
}

// src/context.rs:19
pub struct EmitOwned {
    pub line: Line,
    pub role: Role,
    pub match_info: MatchInfo,
}
```

`EmitOwned` is what flows through the expander's callback; `Emit` is what
formatters see.

---

## 6. Module Reference

Concise role summary for every public module. Source files are at
`src/<path>` in the repo.

| Module | Role | Key types |
|---|---|---|
| `lib.rs` | Crate root, re-exports | `App`, `Selector`, `Result`, `run` |
| `main.rs` | Binary entry point | — |
| `cli.rs` | `clap` definitions + CLI → App wiring | `Cli`, `ColorMode` |
| `app.rs` | Typestate builder | `App<K>`, `Stage1..5`, `Seek`, `NonSeek` |
| `pipeline.rs` | Single driver `run::<K>(App<K>)` | `run` |
| `selector.rs` | Selector parser & normalizer | `Selector`, `LineSpec`, `Position` |
| `context.rs` | Emit planner with optional context | `Expander`, `NoContext`, `LineContext` |
| `types.rs` | Shared value types | `Line`, `MatchInfo`, `Role`, `Emit` |
| `error.rs` | Typed errors | `SelError`, `Result` |
| `matcher/mod.rs` | `Matcher` trait + `AllMatcher` | `Matcher`, `AllMatcher` |
| `matcher/lines.rs` | Line-range matcher | `LineMatcher` |
| `matcher/position.rs` | `L:C` matcher (seekable only) | `PositionMatcher` |
| `matcher/regex.rs` | `regex::bytes::Regex` wrapper | `RegexMatcher` |
| `source/mod.rs` | `Source` trait | `Source` |
| `source/file.rs` | File-backed source | `FileSource` |
| `source/stdin.rs` | Stdin-backed source | `StdinSource` |
| `format/mod.rs` | `Formatter` trait + options | `Formatter`, `FormatOpts` |
| `format/plain.rs` | Line-oriented formatter | `PlainFormatter` |
| `format/fragment.rs` | Char-context fragment formatter | `FragmentFormatter` |
| `format/ansi.rs` | Three ANSI codes + `paint()` | `GREEN`, `INVERSE`, `RESET` |
| `sink/mod.rs` | `Sink` trait | `Sink` |
| `sink/stdout.rs` | Buffered stdout sink | `StdoutSink` |
| `sink/file.rs` | Create-new/force file sink | `FileSink` |

---

## 7. Selector Grammar

### 7.1 Informal grammar

```
selector   ::= all | line_list | pos_list
all        ::= ε                         # empty -> Selector::All
line_list  ::= line_spec ("," line_spec)*
line_spec  ::= number | number "-" number
pos_list   ::= position ("," position)*
position   ::= number ":" number
number     ::= [1-9][0-9]*               # must be >= 1
```

Mixing line specs and positions in the same selector is **rejected**; the
parser picks one variant based on whether the string contains `:`.

### 7.2 Semantics

- **Ranges are inclusive** (`10-20` selects lines 10..=20).
- **Line numbers are 1-indexed**. `0` is rejected.
- **Column numbers are 1-indexed** and measured in **bytes** (important for
  multi-byte UTF-8).
- **Reversed ranges** (`20-10`) are rejected.
- **Duplicates** are removed; adjacent/overlapping ranges are merged via
  `Selector::normalize()`.

### 7.3 What is "selector-ish"?

`Cli::looks_like_selector` (at [`src/cli.rs:140`](../src/cli.rs)) defines a
conservative test so the first positional argument can be either a selector
or a filename without ambiguity:

- Must contain at least one digit.
- Must contain only digits, commas, colons, hyphens.
- Colons must separate non-empty number pairs.
- `-` alone is the stdin sentinel, not a selector.

Filenames like `file.txt`, `10-20.log`, or `23:notes` are **not** matched
because they contain letters or dots.

### 7.4 Example normalizations

| Input | Normalized |
|---|---|
| `1,5,10-15,14` | `1, 5, 10-15` |
| `1,2,3` | `1-3` |
| `1-5,6-10` | `1-10` (adjacent merge) |
| `1-5,3-10` | `1-10` (overlap merge) |
| `5,5,5` | `5` |
| `1-5,10-15` | `1-5, 10-15` (no merge) |

Positions are de-duplicated and sorted, but not merged (each position is
point-valued).

---

## 8. Matchers

Every matcher implements:

```rust
pub trait Matcher {
    fn match_line(&mut self, line: &Line) -> MatchInfo;
}
```

The `&mut self` permits stateful matchers like `PositionMatcher` (which
keeps a cursor into its sorted `positions` list).

### 8.1 `AllMatcher`

Trivial: every line is a hit. Used when the user runs `sel file.txt` with
no selector and no regex.

### 8.2 `LineMatcher`

Owns a `Vec<(u64, u64)>` of merged inclusive ranges. Each line is checked
with a linear scan; for realistic selectors (one to tens of ranges after
normalization), this is faster than a `BTreeSet` or binary search because of
cache locality and branch prediction. See the benchmark
[`benches/large_file.rs:77`](../benches/large_file.rs) for the comparison.

### 8.3 `PositionMatcher`

Carries a sorted, deduped `Vec<Position>` and a `cursor` index. On each
line it advances the cursor past stale entries (positions with `line < current`),
then peeks at `positions[cursor]` to see if it matches the current line.
This is effectively a two-pointer merge of input line numbers (monotonic)
and target positions (also monotonic after `sort`), giving **O(n + p)**
where `n` is the number of input lines and `p` is the number of positions.

### 8.4 `RegexMatcher`

Wraps `regex::bytes::Regex` so it can match UTF-8-invalid bytes. When the
match is a hit (and not inverted), it collects all `Match` spans into
`MatchInfo.spans` for the formatter to highlight. Inverted matches have no
spans (there is nothing to highlight).

The regex dialect is the standard Rust `regex` crate syntax — not PCRE, but
similar; lookaround and backreferences are unsupported by design (linear
time guarantee).

---

## 9. Context Expansion

### 9.1 Two expanders

- **`NoContext`** — passthrough. Emits only hits with `Role::Target`.
- **`LineContext(n)`** — for each hit, emits up to `n` preceding "before"
  lines, the hit itself, then up to `n` trailing lines. Windows are merged
  automatically: if two hits are within `2n+1` of each other, their context
  regions overlap and the expander deduplicates by line number.

### 9.2 Why a ring buffer?

`LineContext` maintains a `VecDeque<(Line, MatchInfo)>` of the last `n`
lines seen. On a hit, it flushes the deque as `Role::Context` then emits
the hit, then switches to "trailing" mode counting down `n` lines that
will be emitted directly as `Context`. This avoids any seek or two-pass
I/O, which is crucial for stdin support.

### 9.3 Overlap handling

`LineContext` tracks `last_emitted: u64` (highest line number emitted so
far) and refuses to re-emit any line number at or below it. This makes
overlap-merging free: two hits `h1` and `h2` with `h2 - h1 <= n` simply
produce a continuous band `[h1 - n, h2 + n]` without duplicates.

Unit tests in [`src/context.rs:118`](../src/context.rs) exercise these
cases (no-dup overlap, symmetric around a hit, drain behaviour).

### 9.4 EOF and `drain`

On EOF, the driver calls `expander.drain(callback)`. For `NoContext` it is
a no-op. For `LineContext`, trailing lines have already been emitted as
they arrived, and any still-buffered "before" lines (those that never
turned into context for a hit) are dropped — they do not belong in the
output.

---

## 10. Formatters & ANSI

### 10.1 `FormatOpts`

```rust
// src/format/mod.rs:20
pub struct FormatOpts {
    pub show_line_numbers: bool,
    pub show_filename: bool,
    pub filename: Option<String>,
    pub color: bool,
    pub target_marker: bool, // "> " on Role::Target (context-aware output)
}
```

`FormatOpts::prefix(line_no)` produces `filename:line:` in shared style.

### 10.2 `PlainFormatter`

Writes one line per emit: `[marker][prefix]content\n`.

- `marker` is `"> "` painted green when `target_marker && role == Target`.
- `content` is the line bytes; if color is on and there are `spans`, each
  span is wrapped in `INVERSE…RESET`.

See [`src/format/plain.rs`](../src/format/plain.rs); unit tests at the
bottom cover prefix, marker, and span painting.

### 10.3 `FragmentFormatter`

Active when `-n N` is present (positional selectors or regex-with-char-
context). Emits a byte-window `bytes[col-N .. col+N]` plus a caret line
aligned under the target column:

```
4:cdefg
  ^
```

If color is on and the current hit has regex spans, the span within the
fragment is highlighted. See [`src/format/fragment.rs`](../src/format/fragment.rs).

### 10.4 ANSI module

```rust
pub const GREEN:   &str = "\x1b[32m";
pub const INVERSE: &str = "\x1b[7m";
pub const RESET:   &str = "\x1b[0m";

pub fn paint(enabled: bool, code: &str, text: &str) -> String;
```

That is the entire color implementation. The 14-line file in
[`src/format/ansi.rs`](../src/format/ansi.rs) replaces what used to be a
dependency on `termcolor`.

### 10.5 Color detection

`Cli::color_mode()` handles user intent (`--color always|never|auto`).
Terminal detection uses `std::io::IsTerminal` (stable since Rust 1.70).
`cli.rs` passes a resolved boolean `color` into `FormatOpts`, so the
formatter does not re-detect.

---

## 11. Sources & Sinks

### 11.1 `Source` trait

```rust
// src/source/mod.rs:15
pub trait Source {
    fn next_line(&mut self) -> Result<Option<Line>>;
    fn label(&self) -> &str;       // for filename prefix
    fn is_seekable(&self) -> bool; // informational mirror of SourceKind
}
```

Both implementations (`FileSource`, `StdinSource`) use a `BufReader` and
`read_until(b'\n', …)` so lines are discovered without requiring UTF-8
validity. Trailing `\r\n` is normalized to Unix-style.

### 11.2 `FileSource`

Owns a `BufReader<File>` plus the `PathBuf`. I/O errors are wrapped as
`SelError::Io { path, source }` so the offending file shows up in the
error message. `is_seekable()` returns `true` — the crate's typestate
ensures `Seek` only attaches to this variant.

### 11.3 `StdinSource`

Locks stdin by `Box::leak`ing the `Stdin` handle for the lifetime of the
process and calling `.lock()`, producing a `StdinLock<'static>`. This lets
the `BufReader` own the lock without fighting lifetime parameters. The
trade-off is a single intentional leak of one small handle, which is fine
because there's exactly one stdin per process and it lasts as long as the
pipeline.

`is_seekable()` returns `false`; positional selectors on stdin are
rejected at two layers (compile-time `NonSeek` type and runtime
`SelError::PositionalWithStdin`).

### 11.4 `Sink` trait

```rust
// src/sink/mod.rs:12
pub trait Sink: Write {
    fn is_terminal(&self) -> bool;
    fn finish(self: Box<Self>) -> io::Result<()>; // flush + surface errors
}
```

The `finish` takes `Box<Self>` so the pipeline can consume the sink by
value at the end of `run()`.

### 11.5 `StdoutSink`

Locks stdout with the same leak-lock trick as `StdinSource` and wraps it
in a `BufWriter` (64 KiB). `is_terminal()` is cached at construction time.

### 11.6 `FileSink`

Opens the output path with either `create_new(true)` (default) or
`create(true).truncate(true)` (when `force`). An existing file without
`--force` yields `SelError::OutputExists(path)` which in turn becomes:

```
Error: output file already exists: out.txt (use --force to overwrite)
```

### 11.7 Interaction summary

| From CLI | Source | Sink | Notes |
|---|---|---|---|
| file arg | `FileSource` | `StdoutSink` | default |
| `-o out` | `FileSource` | `FileSink` | fails if `out` exists |
| `-o out --force` | `FileSource` | `FileSink` | truncates `out` |
| `-o -` | `FileSource` | `StdoutSink` | explicit stdout |
| no file | `StdinSource` | `StdoutSink` | classic pipe |
| `-` as file | `StdinSource` | `StdoutSink` | same as above |

---

## 12. Error Model

### 12.1 `SelError` variants

```rust
// src/error.rs
pub enum SelError {
    InvalidSelector(String),
    InvalidRegex(String),
    PositionalWithStdin,
    InvertWithoutRegex,
    CharContextWithoutTarget,
    Io { path: String, source: io::Error },
    OutputExists(PathBuf),
}
```

All variants produce human-readable messages via `thiserror`'s `#[error(...)]`.
Every `Io` error always carries the offending path, so messages like
`no such file or directory` become `logs/missing.log: no such file or
directory`.

### 12.2 CLI exit behaviour

`main` prints `Error: {e}` to stderr and exits with status 1 on any
`SelError`. `Cli::validate` catches CLI-level conflicts early (before
building a pipeline) so the user sees a crisp message for
`--invert-match` without `-e`, or `--char-context` without a target.

### 12.3 Library users

Library users receive `sel::Result<T>` and can `match` on `SelError`:

```rust
match sel::pipeline::run(app) {
    Err(sel::SelError::OutputExists(p)) => { /* prompt the user */ }
    Err(e) => { /* generic fallback */ }
    Ok(()) => {}
}
```

---

## 13. CLI Wiring

### 13.1 `Cli` struct

All flags are in [`src/cli.rs:24`](../src/cli.rs). Derived via `clap` with
`#[derive(Parser)]`; no manual `App::new` boilerplate.

Notable methods:

- `get_selector()` — returns `Some(raw)` if `args[0]` looks like a selector
  and we are not in regex mode.
- `get_files()` — returns the file list (falls back to `["-"]` if none).
- `validate()` — checks flag conflicts early.
- `color_mode()` — resolves `--color` against TTY detection.
- `into_app_for_file(path, show_filename)` / `into_app_for_stdin(show_filename)`
  — the heart of the wiring, shown below.

### 13.2 The wiring function

```rust
// Simplified version of cli.rs:257
pub fn into_app_for_file(&self, path, show_filename) -> Result<App<Seek>> {
    let source = FileSource::open(path)?;
    let sink = self.make_sink()?;
    let color = self.resolve_color(sink.is_terminal());
    let opts = FormatOpts { /* show_line_numbers, filename, color, ... */ };

    let stage2 = Stage1::with_seekable_source(Box::new(source));
    let stage3 = if regex.is_some() {
        stage2.with_matcher(Box::new(RegexMatcher::new(pat, invert)?))
    } else if let Some(raw) = self.get_selector() {
        match Selector::parse(&raw)? {
            Selector::All         => stage2.with_matcher(Box::new(AllMatcher)),
            Selector::LineNumbers => stage2.with_matcher(Box::new(LineMatcher::from_selector(&sel))),
            Selector::Positions   => stage2.with_position_matcher(PositionMatcher::from_selector(&sel)),
        }
    } else {
        stage2.with_matcher(Box::new(AllMatcher))
    };

    let stage4 = match self.context {
        Some(n) if n > 0 => stage3.with_expander(Box::new(LineContext::new(n))),
        _                => stage3.with_expander(Box::new(NoContext)),
    };

    let stage5 = if let Some(n) = self.char_context {
        stage4.with_formatter(Box::new(FragmentFormatter::new(opts, n)))
    } else {
        stage4.with_formatter(Box::new(PlainFormatter::new(opts)))
    };

    Ok(stage5.with_sink(sink))
}
```

The stdin variant is the same with `NonSeek` and an early `Err(PositionalWithStdin)`
if the user typed `line:col` against stdin.

### 13.3 Multi-file handling

`main.rs` iterates over `get_files()`. Each iteration builds a **fresh**
`App` and runs it through the pipeline:

```rust
// src/main.rs:19
fn run(cli: Cli) -> sel::Result<()> {
    let files = cli.get_files();
    let show_filename = cli.with_filename || files.len() > 1;
    for path in &files {
        if path.as_os_str() == "-" {
            sel::pipeline::run(cli.into_app_for_stdin(show_filename)?)?;
        } else {
            sel::pipeline::run(cli.into_app_for_file(path, show_filename)?)?;
        }
    }
    Ok(())
}
```

File prefix auto-activates when `files.len() > 1`, matching `grep`'s
behaviour. `-H` forces it on.

---

## 14. Library API & Embedding

`sel` is a regular crate on crates.io; `cargo add sel` makes everything
in `lib.rs` available.

### 14.1 The minimum embedding

```rust
use sel::{App, Stage1, NoContext, PlainFormatter, StdoutSink,
          AllMatcher, FormatOpts};
use sel::source::FileSource;

fn cat_all(path: &std::path::Path) -> sel::Result<()> {
    let source = FileSource::open(path)?;
    let opts = FormatOpts {
        show_line_numbers: true,
        show_filename: false,
        filename: None,
        color: false,
        target_marker: false,
    };
    let app = Stage1::with_seekable_source(Box::new(source))
        .with_matcher(Box::new(AllMatcher))
        .with_expander(Box::new(NoContext))
        .with_formatter(Box::new(PlainFormatter::new(opts)))
        .with_sink(Box::new(StdoutSink::new()));
    sel::pipeline::run(app)
}
```

Every flag the CLI offers is achievable the same way: substitute a
different matcher, expander, formatter, or sink.

### 14.2 Parsing selectors from other input

```rust
use sel::Selector;

let sel = Selector::parse("1,5,10-20").unwrap().normalize();
```

`Selector::parse` is pure (no I/O); it's safe to call on user-supplied
strings, and its errors are `SelError::InvalidSelector(reason)`.

### 14.3 Custom matchers

Implement `sel::Matcher` — for example, a "match even line numbers"
matcher:

```rust
use sel::{Matcher, MatchInfo};

struct EvenOnly;
impl Matcher for EvenOnly {
    fn match_line(&mut self, line: &sel::Line) -> MatchInfo {
        MatchInfo { hit: line.no % 2 == 0, ..MatchInfo::default() }
    }
}
```

Plug it into stage 2 just like the built-ins.

### 14.4 Custom sinks

Anything that is `Write + Sink` works. The sink trait requires two extra
methods beyond `Write` (`is_terminal`, `finish`). A common embedding
pattern is to write to an in-memory buffer for snapshot tests.

---

## 15. Extension Points

If you are adding a feature to `sel`, this is the shortest path:

| What you want | What to change |
|---|---|
| New CLI flag | Add field in [`cli.rs:24`](../src/cli.rs), extend `validate()` if needed, wire into `into_app_for_*`. |
| New matching mode | Implement `Matcher`. Decide if it needs seek → pick stage2 method. Wire into `cli.rs`. |
| New context strategy | Implement `Expander` (`push` + `drain`). |
| New output shape | Implement `Formatter` or extend `FormatOpts`. Consider adding a new variant instead of bit-flags. |
| New output destination | Implement `Sink` (which requires `Write`). |
| New error class | Add a variant to `SelError`. Compile errors guide you to every match site. |

For each change, add an integration test under `tests/` and update
`docs/USAGE.md` if it is user-facing.

---

## 16. Performance Characteristics

### 16.1 Complexity

- **Per-line work** is **O(1)** for all matchers in realistic configurations:
  - `AllMatcher`: O(1).
  - `LineMatcher`: O(r) where r = number of merged ranges (small in practice).
  - `PositionMatcher`: amortized O(1) via two-pointer cursor.
  - `RegexMatcher`: O(|line|) via the linear-time Rust regex.
- **Memory** is O(n + L) where n is `-c` and L is the longest line. No
  whole-file buffering.
- **I/O** is one forward pass through `BufReader` (default 8 KiB buffer for
  sources; 64 KiB for the sink).

### 16.2 Release profile

```toml
# Cargo.toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

LTO plus single codegen unit plus panic=abort give tight, small binaries
(important for the `cargo-dist`-published artifacts on the Releases page).

### 16.3 Benchmarks

Criterion benches live in [`benches/large_file.rs`](../benches/large_file.rs):

- `selector_parsing` — parse cost for selectors from "single" to
  "large_mixed".
- `selector_normalize` — cost of `Selector::normalize()` on overlapping /
  adjacent / singleton cases.
- `line_matching` — pre-normalized vs. non-normalized range lookup per
  line (quantifies the normalization win).
- `large_file` — parsing multi-thousand-line selectors.

Run with `cargo bench`. Reports land in `target/criterion/`.

### 16.4 Where bottlenecks live

In practice the hot path is:

1. `BufReader::read_until` — kernel I/O, fairly irreducible.
2. `Regex::is_match` (when `-e`) — dominates for regex workloads.
3. `String::from_utf8_lossy` in the formatter — only when actually writing.

There is currently no parallelism; the pipeline is single-threaded by
design (one file = one pass). Multi-file runs are processed sequentially.

---

## 17. Testing Strategy

### 17.1 Test layers

| Layer | Location | Examples |
|---|---|---|
| **Unit** | `#[cfg(test)] mod tests` next to code | selector parsing, expander edge cases, ANSI painting |
| **Integration** | `tests/*.rs` | end-to-end CLI behaviour |
| **Benchmarks** | `benches/large_file.rs` | Criterion micro-benchmarks |
| **Property-based** | `proptest` dev-dependency | (available; used sparingly) |

### 17.2 Integration test files

| File | Covers |
|---|---|
| [`tests/basic.rs`](../tests/basic.rs) | Smoke tests, single line output |
| [`tests/selectors.rs`](../tests/selectors.rs) | Line numbers, ranges, lists |
| [`tests/positions.rs`](../tests/positions.rs) | `L:C` with/without `-n` |
| [`tests/regex.rs`](../tests/regex.rs) | `-e` patterns, highlighting |
| [`tests/invert.rs`](../tests/invert.rs) | `-v` negative matches |
| [`tests/context.rs`](../tests/context.rs) | `-c N` windows & merging |
| [`tests/stdin.rs`](../tests/stdin.rs) | `-` and bare-stdin, rejections |
| [`tests/multi_file.rs`](../tests/multi_file.rs) | Filename prefixes, multiple paths |
| [`tests/output_file.rs`](../tests/output_file.rs) | `-o`, `--force`, `-o -` |

### 17.3 Running tests

```bash
cargo test                       # everything, debug
cargo test --release             # optional, catches release-only issues
cargo test --test regex          # one integration file
cargo test selector::tests       # module subset
cargo test -- --nocapture        # stdout from tests
```

Integration tests run in parallel; they must use `tempfile::TempDir` for
any filesystem interaction.

### 17.4 Coverage philosophy

Error paths are covered in addition to happy paths because the error
messages are part of the user experience. For example,
`positional_with_stdin_has_clear_message` in [`src/error.rs`](../src/error.rs)
checks that the user-visible message contains the word "stdin".

---

## 18. Build, Release & CI

### 18.1 Build matrix

GitHub Actions runs a single matrix job on every push (see
`.github/workflows/ci.yml`) across Linux, macOS, and Windows. The job:

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`

### 18.2 Release pipeline

Tagging `vX.Y.Z` triggers `release.yml`, which is driven by
[`cargo-dist`](https://opensource.axo.dev/cargo-dist/). It:

1. Builds cross-platform binaries for the targets declared in
   `Cargo.toml` `[workspace.metadata.dist]`:
   - `aarch64-apple-darwin`, `x86_64-apple-darwin`
   - `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`
   - `x86_64-pc-windows-msvc`
2. Publishes a GitHub Release with shell and PowerShell installers.
3. Runs `cargo publish` to push the new version to crates.io.

The release workflow is manually tweaked to append the `cargo publish`
step; `allow-dirty = ["ci"]` in `Cargo.toml` lets cargo-dist coexist with
that manual section.

### 18.3 Maintainer release steps

1. Bump `version` in `Cargo.toml`.
2. Move `## [Unreleased]` → `## [X.Y.Z] — DATE` in `CHANGELOG.md`.
3. Commit with `chore: bump to X.Y.Z`.
4. Tag `vX.Y.Z`, push with `--tags`.
5. CI takes over.

---

## 19. Troubleshooting & Pitfalls

### 19.1 Common user-facing errors

| Symptom | Likely cause | Fix |
|---|---|---|
| `Error: invalid selector: '...'` | Non-digit/comma/hyphen in selector | Check selector syntax; use `-e` for patterns |
| `Error: positional selectors require a seekable file; stdin is line-only` | `sel 23:260 -` or `... \| sel 23:260` | Pass a file path, or drop the column |
| `Error: --invert-match requires --regex` | `-v` without `-e` | Add `-e PATTERN` |
| `Error: --char-context requires --regex or a positional selector` | `-n N` with bare line selector | Use `-c N` for line context, or add `-e`/position |
| `Error: output file already exists: ... (use --force to overwrite)` | `-o EXISTING` | Add `--force` or pick another path |

### 19.2 Performance pitfalls

- Very large `-c` values force a larger ring buffer; memory grows O(n·L).
- Catastrophic regex patterns are impossible with the Rust regex crate
  (linear time), but extremely permissive regexes still dominate the
  per-line cost.
- `sel 1-10000000 huge.txt` has to count lines up to the upper bound; the
  matcher itself is fast, but the I/O pass still reads and compares
  every line.

### 19.3 UTF-8 pitfalls

- **Column numbers are byte positions.** A line with multi-byte UTF-8
  before the target column will not align with character counts from a
  text editor. Convert ahead of time if needed.
- **Terminal rendering** depends on the user's terminal; `sel` emits raw
  ANSI and line contents. Control bytes in input will be passed through
  verbatim.

### 19.4 Shell pitfalls

- `!` in a regex may be history-expanded by bash/zsh; quote the pattern:
  `sel -e '!important' notes.txt`.
- Negative-looking patterns (`-foo`) can confuse `clap`; separate with
  `--`: `sel -e -- '-foo' logs/*.log`.

---

## 20. Glossary

**Emit.** A line produced by the expander, carrying metadata (role, spans).
Formatters convert emits to bytes.

**Expander.** Stage between matcher and formatter. Decides what is emitted
and when: just hits, or hits plus context.

**Formatter.** Converts an `Emit` into bytes to write to a `Sink`.

**Hit.** A line that matched the matcher's criterion. `MatchInfo.hit`.

**Line spec.** A single line or a range (`LineSpec::Single | Range`).

**Matcher.** Per-line classifier. Stateful: may remember where it is in a
sorted list of positions.

**Position.** A (line, column) pair.

**Role.** Whether an emit is a target or a context neighbour.

**Seekable.** A source that can be paired with a positional selector.
Only `FileSource` is seekable.

**Selector.** Parsed form of the user's selector argument:
`All | LineNumbers(Vec<LineSpec>) | Positions(Vec<Position>)`.

**Sink.** Output destination: `StdoutSink` or `FileSink`.

**Source.** Input origin: `FileSource` or `StdinSource`. Streams lines.

**Typestate.** The pattern used in `app.rs` where `App<K>` and the stages
use phantom types to encode valid transitions.

---

## Appendix A — File Index

Each path is relative to the repo root.

```
Cargo.toml                 manifest + dist config
README.md                  quickstart for users
CHANGELOG.md               version history
CONTRIBUTING.md            dev loop, test rules, release steps
CODE_OF_CONDUCT.md         community standards
LICENSE-MIT                MIT
LICENSE-APACHE             Apache-2.0

src/main.rs                binary entry point (30 lines)
src/lib.rs                 public re-exports
src/cli.rs                 clap struct, args → App wiring, ColorMode
src/app.rs                 Stage1..5, App<K>, Seek / NonSeek
src/pipeline.rs            the single driver loop
src/selector.rs            Selector, LineSpec, Position, parse/normalize
src/context.rs             Expander, NoContext, LineContext, EmitOwned
src/error.rs               SelError + Result
src/types.rs               Line, MatchInfo, Role, Emit

src/matcher/mod.rs         Matcher trait + AllMatcher
src/matcher/lines.rs       LineMatcher
src/matcher/position.rs    PositionMatcher (seek-only)
src/matcher/regex.rs       RegexMatcher (bytes regex)

src/source/mod.rs          Source trait
src/source/file.rs         FileSource
src/source/stdin.rs        StdinSource

src/format/mod.rs          Formatter, FormatOpts
src/format/plain.rs        PlainFormatter
src/format/fragment.rs     FragmentFormatter (char-context + caret)
src/format/ansi.rs         three ANSI codes + paint()

src/sink/mod.rs            Sink trait
src/sink/stdout.rs         StdoutSink (64 KiB BufWriter)
src/sink/file.rs           FileSink (create-new / force)

tests/                     per-feature integration tests
benches/large_file.rs      Criterion micro-benchmarks

docs/README.md             user docs index
docs/USAGE.md              short CLI reference
docs/ARCHITECTURE.md       short architectural summary
docs/REFERENCE.md          (this file) the long-form reference
```

---

## Appendix B — Reading Paths

Suggested traversals for different audiences.

### For a user who just installed `sel`

1. [`README.md`](../README.md) — quick examples.
2. [`USAGE.md`](USAGE.md) — every flag explained once.
3. Sections [7](#7-selector-grammar) and [19](#19-troubleshooting--pitfalls)
   of this file.

### For a new contributor

1. [`CONTRIBUTING.md`](../CONTRIBUTING.md) — dev loop & conventions.
2. Sections [2](#2-system-overview), [3](#3-design-decisions), and
   [4](#4-the-pipeline-in-detail) of this file.
3. [`src/pipeline.rs`](../src/pipeline.rs), [`src/cli.rs`](../src/cli.rs),
   [`src/app.rs`](../src/app.rs) — three files, three concerns.
4. [`tests/basic.rs`](../tests/basic.rs) to see how end-to-end tests
   look; then pick a feature area matching your change and read its test
   file.

### For someone embedding `sel` as a library

1. Section [14](#14-library-api--embedding) of this file.
2. [`src/lib.rs`](../src/lib.rs) — the full re-export surface.
3. rustdoc on [docs.rs/sel-rs](https://docs.rs/sel-rs).
4. Section [15](#15-extension-points) when adding a custom stage.

### For an architect reviewing the design

1. Sections [3](#3-design-decisions) and [4](#4-the-pipeline-in-detail).
2. Sections [8](#8-matchers)–[11](#11-sources--sinks) for stage-level
   detail.
3. Section [16](#16-performance-characteristics) for complexity & bench
   methodology.

---

*Last regenerated: see the git history of this file.*
