# Contributing to `sel`

Thanks for your interest in improving `sel`! This document covers the local
development loop, project layout, review process, and release flow.

**Docs index:** [docs/README.md](docs/README.md) (usage, architecture, links to
API docs on docs.rs).

## Code of Conduct

This project adopts the [Contributor Covenant](CODE_OF_CONDUCT.md). By
participating you agree to abide by its terms.

## Getting the code

```bash
git clone https://github.com/InkyQuill/sel.git
cd sel
cargo build
```

Minimum supported Rust version (MSRV): **1.92** (2024 edition), as declared in
[`Cargo.toml`](Cargo.toml).

## Development loop

```bash
cargo fmt                                         # format
cargo clippy --all-targets -- -D warnings         # lint (CI treats warnings as errors)
cargo test                                        # unit + integration + doctests
cargo test --release                              # optional, catches release-only issues
cargo bench                                       # optional, criterion benches in benches/
```

All three of `cargo fmt --check`, `cargo clippy … -D warnings`, and
`cargo test` must be green on Linux, macOS, and Windows before a PR is merged
(see [`.github/workflows/ci.yml`](.github/workflows/ci.yml)).

## Project layout

```
src/
├── main.rs            Thin entry point; parses CLI, dispatches each file through
│                      the pipeline.
├── lib.rs             Re-exports the public API.
├── cli.rs             `clap` definitions + `Cli → App` wiring (per file / stdin).
├── app.rs             Typestate builder (`Stage1…Stage5 → App`). Encodes the
│                      invariant that positional selectors require a seekable
│                      source.
├── pipeline.rs        Single driver `run::<K>(App<K>)` that streams lines
│                      through matcher → expander → formatter → sink.
├── selector.rs        Parse/normalise `Selector` (lines, ranges, positions).
├── context.rs         `Expander` trait: `NoContext` and `LineContext` (merged
│                      `-c N` windows).
├── error.rs           `SelError`, typed via `thiserror`.
├── types.rs           `Line`, `MatchInfo`, `Role`, `Emit`.
├── matcher/           Per-line classifier: `AllMatcher`, `LineMatcher`,
│                      `PositionMatcher`, `RegexMatcher`.
├── source/            `Source` trait: `FileSource`, `StdinSource`.
├── format/            `Formatter` trait: `PlainFormatter`, `FragmentFormatter`
│                      (char-context), plus ANSI helpers.
└── sink/              `Sink` trait: `StdoutSink`, `FileSink` (refuses to
                       overwrite without `--force`).

tests/                 Black-box integration tests per feature
                       (selectors, regex, context, stdin, multi_file, …).

benches/large_file.rs  Criterion micro-benchmarks for the streaming path.
docs/                  User docs: README index, USAGE, ARCHITECTURE;
                       superpowers/ holds optional design session notes.
```

See [`docs/README.md`](docs/README.md) for the full documentation index and
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the pipeline design.

## Writing tests

- Prefer **integration tests** under `tests/` — they exercise the binary
  through `assert_cmd` / in-process `App`s and document end-user behaviour.
- Keep unit tests next to the code they cover (`#[cfg(test)] mod tests`).
- For properties of the selector parser and context expander, consider adding a
  [`proptest`](https://github.com/proptest-rs/proptest) case (already a
  dev-dependency).
- Test both **success** and **error** paths. Error messages are part of the UX
  and should be covered.
- Avoid touching files outside `tempfile::TempDir` — tests run in parallel.

Run an individual suite with:

```bash
cargo test --test regex -- position
cargo test selector::tests::parse_position
```

## Commit convention

This project follows [Conventional Commits](https://www.conventionalcommits.org/).
Prefixes we use:

| prefix      | meaning                                          |
|-------------|--------------------------------------------------|
| `feat:`     | new user-visible feature                         |
| `fix:`      | bug fix                                          |
| `perf:`     | performance improvement                          |
| `refactor:` | internal change with no behaviour change         |
| `docs:`     | README/CONTRIBUTING/rustdoc/comments             |
| `test:`     | added or refined tests                           |
| `chore:`    | tooling, formatting, dependency bumps            |
| `ci:`       | GitHub Actions / release pipeline                |

Scopes are optional. Breaking changes go in a `BREAKING CHANGE:` footer (or a
`!` after the type) and trigger a major version bump under semver.

Examples:

```
feat: support `-o -` as an explicit stdout sentinel
fix: handle CRLF newlines in position matcher
refactor(pipeline): drop unused EmitOwned alias
```

## Pull request checklist

Before opening a PR:

- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.
- [ ] `cargo test` is green.
- [ ] New features come with integration tests under `tests/`.
- [ ] Rustdoc comments on new public items.

`Cargo.toml` and `CHANGELOG.md` are maintained automatically by
[release-plz](https://release-plz.ieni.dev/) based on your commit messages.
You do **not** need to bump the version or edit the `[Unreleased]` section
manually — just write clear Conventional Commits.

Keep PRs focused — one change, one PR. Prefer smaller diffs with clear intent
over sweeping rewrites.

## Release process (maintainers)

Releases are driven by [release-plz](https://release-plz.ieni.dev/) +
[cargo-dist](https://opensource.axo.dev/cargo-dist/):

1. After any `feat:` / `fix:` / `perf:` / `refactor:` commit lands on `main`,
   the [`release-plz.yml`](.github/workflows/release-plz.yml) workflow opens
   or updates a **release PR** titled something like `chore: release v0.3.0`.
   The PR bumps `Cargo.toml` and rewrites the `CHANGELOG.md` section for the
   upcoming version using commit messages since the last tag.
2. Review the release PR. Edit the CHANGELOG body in the PR if you want to
   polish it. Merge when you're ready to ship.
3. Merging the release PR triggers release-plz's `release` job, which pushes
   the `vX.Y.Z` tag.
4. The tag push fires [`release.yml`](.github/workflows/release.yml)
   (cargo-dist), which builds cross-platform binaries, publishes a GitHub
   Release, and runs `cargo publish` to crates.io.

### One-time setup

The release-plz workflow needs a Personal Access Token (classic, `repo`
scope) stored as a repo secret named `RELEASE_PLZ_TOKEN`. A token is needed
(rather than the default `GITHUB_TOKEN`) because GitHub deliberately
prevents pushes made with `GITHUB_TOKEN` from triggering downstream
workflows — the release tag would land but cargo-dist would never fire.

`CARGO_REGISTRY_TOKEN` is still required by cargo-dist's
`publish-crates-io` job.

## Reporting bugs / requesting features

Open an issue with:

- `sel --version` output,
- exact command you ran,
- input that reproduces the behaviour (or a minimal synthetic one),
- actual vs. expected output.

## License of contributions

By contributing, you agree that your work is dual-licensed under
**MIT OR Apache-2.0**, matching the project license (see
[`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE)).
