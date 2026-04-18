# `prefer-sel-over-sed` — a Claude skill for the `sel` tool

A small [Claude Code skill](https://docs.claude.com/en/docs/claude-code/skills)
that tells the model to prefer the `sel` CLI over `sed -n '…p'` for
line / range / regex extraction when `sel` is installed.

## What this contains

| Path | What it is |
|---|---|
| `prefer-sel-over-sed/SKILL.md` | The skill itself (Markdown + YAML frontmatter). |
| `install.sh` | POSIX shell installer. Copies the skill into a chosen Claude skills folder. |

## Installing

### One-liner (no checkout required)

```bash
# User-level install (~/.claude/skills/prefer-sel-over-sed)
curl -fsSL https://raw.githubusercontent.com/InkyQuill/sel/main/contrib/claude-skill/install.sh | sh

# Project-level install into the current directory (./.claude/skills/...)
curl -fsSL https://raw.githubusercontent.com/InkyQuill/sel/main/contrib/claude-skill/install.sh | sh -s -- --project

# wget works too
wget -qO- https://raw.githubusercontent.com/InkyQuill/sel/main/contrib/claude-skill/install.sh | sh
```

Pin to a specific release with `SEL_SKILL_REF`:

```bash
curl -fsSL https://raw.githubusercontent.com/InkyQuill/sel/main/contrib/claude-skill/install.sh \
  | SEL_SKILL_REF=v0.2.0 sh
```

### From a checked-out repo

```bash
./contrib/claude-skill/install.sh              # user-level (default)
./contrib/claude-skill/install.sh --project    # project-level, CWD
./contrib/claude-skill/install.sh --project ~/code/some-app
./contrib/claude-skill/install.sh --force      # overwrite existing install
./contrib/claude-skill/install.sh --uninstall  # remove (default: user-level)
./contrib/claude-skill/install.sh --uninstall --project
```

Run `./contrib/claude-skill/install.sh --help` for the full flag list.

The installer only needs `/bin/sh`, `cp`, `mkdir`, `rm`, and (for the
one-liner path) `curl` or `wget`. It works on Linux, macOS, and WSL.
Windows users without WSL can copy the `prefer-sel-over-sed/` directory
manually into `%USERPROFILE%\.claude\skills\`.

## What the skill does

When Claude is about to write a shell command like:

```bash
sed -n '10,20p' file
sed -n '1p;5p;10,15p' file
sed -n '/ERROR/p' file
```

…and `sel` is available, the skill nudges Claude to prefer:

```bash
sel 10-20 file
sel 1,5,10-15 file
sel -e ERROR file
```

It stays out of the way for `sed` calls that do substitution, deletion,
or hold-space tricks — `sel` is deliberately read-only and line-oriented.

See [`prefer-sel-over-sed/SKILL.md`](prefer-sel-over-sed/SKILL.md) for
the full translation table and caveats.

## Requirements

The skill only activates when `sel` is on `PATH`. Install with:

```bash
cargo install sel
# or grab a pre-built binary from the Releases page:
# https://github.com/InkyQuill/sel/releases
```
