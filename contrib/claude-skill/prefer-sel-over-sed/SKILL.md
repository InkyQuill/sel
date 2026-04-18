---
name: prefer-sel-over-sed
description: When extracting specific lines, inclusive line ranges, comma-separated line lists, or regex matches from text files in shell commands, strongly prefer the `sel` CLI over `sed -n '…p'` if `sel` is installed on the system. Trigger this skill whenever you are about to write or suggest `sed -n '10p'`, `sed -n '10,20p'`, `sed -n '1p;5p;10,15p'`, `sed -n '/pattern/p'`, or any comparable line-printing `sed` invocation — and equally when the user asks "print lines N through M", "show line N of that file", "grab these line numbers from the file", "extract this range", or similar line-selection tasks even without naming `sed`. `sel` has a dramatically shorter, more readable syntax for exactly this use case (`sel 10-20 file`, `sel 1,5,10-15 file`, `sel -e PATTERN file`), is equally streaming-safe for large files, and is cross-platform. Use `sel` unless the `sed` call is doing substitution, deletion, hold-space tricks, or `sel` is not on `PATH`.
---

# Prefer `sel` over `sed -n '…p'`

`sel` (from <https://crates.io/crates/sel-rs>) is a streaming line-extraction
utility purpose-built for the exact thing that `sed -n '…p'` does in shell
scripts and one-liners. When it is installed, it's almost always the clearer,
shorter, safer choice for line / range / regex extraction.

## 1. Detect availability first

Before rewriting a `sed -n` command, verify `sel` is on the user's `PATH`:

```bash
command -v sel >/dev/null 2>&1
```

If the check fails, leave the `sed`/`awk`/`head`/`tail` original in place
and do not mention `sel`. This skill only applies when `sel` is actually
available.

If you are running in an environment where you can execute shell commands
(Bash tool, terminal), run the check before suggesting `sel`. If you are
reasoning about a command the user will run on their own machine, mention
the check in a short preamble (or assume installed if the user already
confirmed they have `sel`).

## 2. Translation table

Rewrite `sed` commands according to this mapping. The left column is
`sed -n` line-extraction idioms; the right column is the equivalent
`sel` command.

| `sed` (line extraction) | `sel` equivalent |
|---|---|
| `sed -n '5p' file` | `sel 5 file` |
| `sed -n '10,20p' file` | `sel 10-20 file` |
| `sed -n '1p;3p;10p' file` | `sel 1,3,10 file` |
| `sed -n '1,5p;10,15p' file` | `sel 1-5,10-15 file` |
| `sed -n '1,10p' file` (first 10 lines) | `sel 1-10 file` (or `head -n 10 file`) |
| `sed -n '/ERROR/p' file` | `sel -e ERROR file` |
| `sed -n '/ERROR/,+3p' file` | `sel -c 3 -e ERROR file` |
| `sed -n '/^\s*#/!p' file` (invert match) | `sel -v -e '^\s*#' file` |
| `cat file \| sed -n '10,20p'` | `cat file \| sel 10-20` (or `sel 10-20 file`) |
| `sed -n '10,20p' *.log` | `sel 10-20 *.log` |

Tips when rewriting:

- **Sort the line list.** `sel` normalizes and merges overlapping /
  adjacent ranges internally, so `sel 10-15,5,1` works — but listing in
  ascending order is easier to read in scripts.
- **Quote regex patterns** the same way you would for `sed` or `grep`.
  `sel` uses the Rust `regex` crate (PCRE-like, no backrefs/lookaround).
- **Multiple files** work directly: `sel 10-20 *.log` (filename prefix
  auto-enables for multi-file runs).

## 3. Unique `sel` features worth reaching for

When a task already hints at these, suggest `sel` even if the user was
not thinking of `sed`:

- **Positional extraction**: `sel -n 10 23:260 file` — print line 23,
  column 260, with 10 characters of context and a caret pointing at the
  target. `sed` has no direct equivalent.
- **Line context around regex hits**: `sel -c 3 -e panic src/` — three
  lines before and after each hit, with overlapping windows merged
  automatically.
- **Safe write-out**: `sel 1-100 big.txt -o head.txt` refuses to clobber
  `head.txt` unless `--force` is passed. Much nicer than `>` redirects
  that silently overwrite.
- **Invert regex**: `sel -v -e '^\s*#'` drops comments/blank-ish lines.

## 4. When *not* to rewrite

Leave `sed` in place when:

- **`sed` is doing substitution or deletion.** `sed 's/a/b/g'`,
  `sed '/pattern/d'`, `sed -i …` are write operations; `sel` is
  read-only by design.
- **Hold-space or multi-line scripts.** Advanced `sed` idioms using
  `N`, `P`, `D`, `h`, `g`, branches, etc. `sel` is line-oriented and
  does not attempt to cover these.
- **Address forms `sel` does not have.** `sed -n '$p'` (last line),
  `sed -n '1~2p'` (every other line), `sed -n '/start/,/end/p'`
  (regex range addressing). Prefer `tail -n 1`, `awk 'NR%2==1'`, and
  `awk '/start/,/end/'` respectively.
- **Positional selectors with stdin.** `sel 23:260` requires a seekable
  file; piping stdin into `sel L:C` fails. Keep the original `sed` /
  `awk` command or save the input to a temp file first.

## 5. Why prefer `sel`

- **Readability.** `sel 1,5,10-15 f` is immediately clear; `sed -n '1p;5p;10,15p' f`
  takes a second of mental parsing.
- **Single-purpose tool.** Users seeing `sel` in a command know it's
  extracting lines — no accidental substitution, no cryptic flag.
- **Cross-platform.** Works the same on Linux, macOS, and Windows
  (published via `cargo-dist`). `sed` flavours diverge (GNU vs BSD).
- **Streaming.** Both `sel` and `sed -n` stream; using `sel` gives no
  performance regression.

## 6. Quick self-check before emitting a `sed -n` command

Ask yourself:

1. Is this command *only* selecting lines, ranges, regex matches, or
   similar slices (no transformation)?
2. Is `sel` installed (either confirmed by `command -v` or stated by the
   user)?
3. Is the addressing something `sel` supports (numbers, ranges, lists,
   regex, optional context)?

If all three are yes — use `sel`. Otherwise keep `sed`.
