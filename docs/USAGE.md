# CLI usage

**sel** extracts fragments from text by line numbers, ranges, comma-separated
lists, positions (`line:column`), or regex. It streams input line-by-line so
large files stay practical.

For the exact help text your binary was built with, run:

```bash
sel --help
```

## Modes

### Line / list / range selectors

When **not** using `-e`, the first positional argument may be a **selector** if
it matches selector syntax (digits, commas, hyphens, optional colon for
position). Remaining positionals are input paths. If the first argument does not
look like a selector, it is treated as a file (and stdin is not implied unless
you use `-` or omit files per rules below).

Examples:

```bash
sel 30-35 file.txt
sel 1,5,10-15,22 file.txt
sel -c 3 42 file.txt
sel -n 10 23:260 file.txt
```

### Regex mode (`-e`)

With `-e PATTERN`, the selector is ignored. **All** positional arguments are
input files. Use `-v` to invert the match (lines that do **not** match).

```bash
sel -e 'TODO|FIXME' src/*.rs
sel -v -e '^\s*#' config.ini
```

### Whole file with line numbers

If you pass only a file (no selector), **sel** prints the entire file with line
numbers (similar to `cat -n`).

```bash
sel file.txt
```

## Input sources

- **Explicit paths**: one or more files after the optional selector.
- **Stdin**: use `-` as a filename, or provide **no** file arguments (stdin is
  then used). In regex mode, no files means stdin.
- **Positional selectors (`line:column`)** require a **seekable** file; pairing
  them with stdin is rejected with a clear error (the implementation cannot
  jump to arbitrary lines on a pipe).

## Output

- Default: **stdout**.
- **`-o FILE`**: write to `FILE`. Existing files are **not** overwritten unless
  you add **`--force`**. Use **`-o -`** to mean stdout explicitly.
- **`-l` / `--no-line-numbers`**: omit line numbers in the body (filenames may
  still appear for multi-file runs).
- **`-H` / `--with-filename`**: always prefix lines with the source path (like
  `grep -H`); by default the prefix appears only when multiple inputs are
  processed.

## Context

- **`-c N`**: include `N` lines before and after each match. Overlapping context
  windows are merged.
- **`-n N`**: character context around a **position** (`L:C`) or around regex
  matches when applicable.

## Colors

**`--color auto|always|never`**

- `auto` (default): color when stdout is a terminal.
- ANSI highlighting marks matches and structural cues where supported.

## Options summary

| Option | Meaning |
|--------|---------|
| `-c`, `--context N` | Line context around matches |
| `-n`, `--char-context N` | Character context (positions / regex) |
| `-l`, `--no-line-numbers` | Hide line numbers |
| `-e`, `--regex PATTERN` | Regex mode; all args are files |
| `-v`, `--invert-match` | With `-e`, emit non-matching lines |
| `-H`, `--with-filename` | Always print `file:` prefix |
| `--color WHEN` | `auto`, `always`, or `never` |
| `-o`, `--output FILE` | Write to file (`-` = stdout) |
| `--force` | Allow `-o` to replace an existing file |
| `-h`, `--help` | Help |
| `-V`, `--version` | Version |

## Selector syntax (informal)

- **Line**: positive integer, e.g. `42`.
- **Range**: `M-N` inclusive.
- **List**: comma-separated pieces, e.g. `1,5,10-15`.
- **Position**: `line:column` (1-based line, 1-based column).

Regex patterns use the Rust `regex` crate (PCRE-like features; see upstream
docs for exact syntax).

## Exit status

Non-zero exits indicate I/O failures, invalid CLI combinations, or other
reported `SelError` conditions; run with a failing case locally to see the
message your version prints.
