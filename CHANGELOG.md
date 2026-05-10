# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] — 2026-05-10

### Fixed
- Align line numbers and reuse output sink


## [0.2.0] — 2026-04-18

### Added
- Read from stdin when no file is given or when `-` is used as a filename.
- `-o`/`--output FILE` writes to a file; fails if the file exists unless `--force` is passed. Use `-o -` to force stdout.
- `-v`/`--invert-match` emits lines that do NOT match `-e PATTERN`.
- Full user docs under `docs/`: `USAGE.md`, `ARCHITECTURE.md`, `REFERENCE.md` (long-form technical reference), plus a `docs/README.md` index.
- `CODE_OF_CONDUCT.md` (Contributor Covenant) and rewritten `CONTRIBUTING.md` with dev loop, test rules, and release workflow.
- Dual-licence `LICENSE-MIT` / `LICENSE-APACHE` files alongside the existing `MIT OR Apache-2.0` manifest declaration.
- `contrib/claude-skill/`: a Claude Code skill that nudges the model to prefer `sel` over `sed -n '…p'` for line extraction, plus a `curl | sh` installer.

### Changed
- Internal refactor: every run is now a single five-stage pipeline (Source → Matcher → Expander → Formatter → Sink) driven by one generic `pipeline::run()`. `main.rs` shrunk from ~600 lines to ~30.
- Typed `App` builder makes positional selectors with stdin a compile-time error; CLI catches the same with a clear runtime message.
- `SelError::Io` now always carries the offending file path.
- Release pipeline migrated to `cargo-dist` + crates.io auto-publish on tag.
- `sel --version` now reports the actual package version from `CARGO_PKG_VERSION` instead of a hardcoded string.

### Removed
- `anyhow`, `termcolor`, `is-terminal` dependencies (unused or subsumed by std).
- Legacy `Message` and `FileNotFound` error variants (replaced by specific variants).

## [0.1.0] — 2026-02-26

### Added
- Initial release of `sel` — Select Slices utility.
- Line number selectors (`N`, `M-N`, `N1,N2,M1-M2`).
- Positional selectors (`L:C`) with character context.
- Regex search support with `-e` flag.
- Line context with `-c` flag.
- Character context with `-n` flag.
- Color output support with `--color`.
- Multiple file support.
- Streaming file reading for large files.
- Comprehensive test suite.
