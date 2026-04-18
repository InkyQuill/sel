#!/usr/bin/env sh
# install.sh — install the `prefer-sel-over-sed` Claude skill.
#
# The skill tells Claude to prefer `sel` over `sed -n '…p'` for
# line / range / regex extraction when `sel` is installed.
#
# Works both when run from a checked-out repo (copies the local
# SKILL.md) and when piped from the internet (fetches SKILL.md
# from GitHub). By default it installs to the user-level skills
# folder (~/.claude/skills/). Use --project to install into the
# current project's .claude/skills/ instead, or --dest PATH for a
# custom parent directory.
#
# Remote source override:
#   SEL_SKILL_REPO (default: InkyQuill/sel)
#   SEL_SKILL_REF  (default: main)

set -eu

SKILL_NAME="prefer-sel-over-sed"
REPO="${SEL_SKILL_REPO:-InkyQuill/sel}"
REF="${SEL_SKILL_REF:-main}"
REMOTE_BASE="https://raw.githubusercontent.com/$REPO/$REF/contrib/claude-skill/$SKILL_NAME"

# Try to resolve the directory this script lives in. When the script is
# piped through `sh` (e.g., `curl … | sh`) $0 will be `sh` rather than a
# real file — that's fine, SCRIPT_DIR simply won't contain a valid
# SKILL_SRC and we'll fall back to the remote fetch path below.
script_path="$0"
if command -v readlink >/dev/null 2>&1; then
    script_path=$(readlink -f -- "$0" 2>/dev/null || printf '%s\n' "$0")
fi
if [ -f "$script_path" ]; then
    SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$script_path")" && pwd -P)
else
    SCRIPT_DIR=""
fi
SKILL_SRC="${SCRIPT_DIR:+$SCRIPT_DIR/}$SKILL_NAME"

print_help() {
    cat <<EOF
Install the '$SKILL_NAME' Claude skill.

USAGE
  install.sh [--user | --project [PATH] | --dest PATH]
  install.sh --uninstall [--user | --project [PATH] | --dest PATH]
  install.sh --help

DEFAULT
  Without arguments, installs into the user-level Claude skills folder:
    \$HOME/.claude/skills/$SKILL_NAME

OPTIONS
  --user           Install into \$HOME/.claude/skills/$SKILL_NAME (default).
  --project [DIR]  Install into DIR/.claude/skills/$SKILL_NAME. If DIR is
                   omitted, the current working directory is used.
  --dest DIR       Install into DIR/$SKILL_NAME. DIR is expected to be a
                   Claude 'skills' folder (user, project, or otherwise).
  --uninstall      Remove the skill from the target location instead of
                   installing it. Accepts the same target flags.
  --force          Overwrite an existing install without prompting.
  --help, -h       Print this help and exit.

REMOTE SOURCE (when not run from a checked-out repo)
  SEL_SKILL_REPO   GitHub 'owner/repo' slug (default: $REPO).
  SEL_SKILL_REF    Branch, tag, or commit (default: $REF).

EXAMPLES
  install.sh                         # Global install for the current user.
  install.sh --project               # Install into ./.claude/skills/...
  install.sh --project ~/code/app    # Install into ~/code/app/.claude/skills/...
  install.sh --uninstall --user      # Remove the global install.

  # One-liner from the internet:
  curl -fsSL https://raw.githubusercontent.com/$REPO/$REF/contrib/claude-skill/install.sh | sh
  curl -fsSL https://raw.githubusercontent.com/$REPO/$REF/contrib/claude-skill/install.sh | sh -s -- --project
EOF
}

mode="user"
dest_override=""
project_dir=""
uninstall=0
force=0

# POSIX-compliant argument parsing.
while [ $# -gt 0 ]; do
    case "$1" in
        --user)
            mode="user"
            shift
            ;;
        --project)
            mode="project"
            shift
            if [ $# -gt 0 ]; then
                case "$1" in
                    --*) : ;;
                    *) project_dir="$1"; shift ;;
                esac
            fi
            ;;
        --dest)
            if [ $# -lt 2 ]; then
                printf 'Error: --dest requires a path argument\n' >&2
                exit 1
            fi
            mode="dest"
            dest_override="$2"
            shift 2
            ;;
        --uninstall)
            uninstall=1
            shift
            ;;
        --force|-f)
            force=1
            shift
            ;;
        --help|-h)
            print_help
            exit 0
            ;;
        *)
            printf 'Unknown argument: %s\n\n' "$1" >&2
            print_help >&2
            exit 1
            ;;
    esac
done

# Resolve the target parent (the '.../skills' directory).
case "$mode" in
    user)
        TARGET_PARENT="$HOME/.claude/skills"
        ;;
    project)
        if [ -z "$project_dir" ]; then
            project_dir=$(pwd)
        fi
        TARGET_PARENT="$project_dir/.claude/skills"
        ;;
    dest)
        TARGET_PARENT="$dest_override"
        ;;
esac

TARGET="$TARGET_PARENT/$SKILL_NAME"

# --- Uninstall path -----------------------------------------------------
if [ "$uninstall" -eq 1 ]; then
    if [ -d "$TARGET" ]; then
        rm -rf -- "$TARGET"
        printf 'Removed %s\n' "$TARGET"
    else
        printf 'Nothing to remove at %s\n' "$TARGET"
    fi
    exit 0
fi

# --- Pre-flight: target collision --------------------------------------
if [ -e "$TARGET" ] && [ "$force" -ne 1 ]; then
    printf 'Target already exists: %s\n' "$TARGET" >&2
    printf 'Pass --force to overwrite, or --uninstall to remove it first.\n' >&2
    exit 1
fi

mkdir -p -- "$TARGET_PARENT"
if [ -e "$TARGET" ]; then
    rm -rf -- "$TARGET"
fi

# --- Install: prefer local source, fall back to remote fetch -----------
fetch() {
    url="$1"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --retry 2 --max-time 30 -- "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- --tries=2 --timeout=30 -- "$url"
    else
        printf 'Error: need `curl` or `wget` to fetch the skill from %s\n' "$url" >&2
        return 1
    fi
}

if [ -n "$SKILL_SRC" ] && [ -d "$SKILL_SRC" ] && [ -f "$SKILL_SRC/SKILL.md" ]; then
    cp -R -- "$SKILL_SRC" "$TARGET"
    source_label="local: $SKILL_SRC"
else
    mkdir -p -- "$TARGET"
    if ! fetch "$REMOTE_BASE/SKILL.md" > "$TARGET/SKILL.md"; then
        printf 'Error: failed to download SKILL.md from %s\n' "$REMOTE_BASE/SKILL.md" >&2
        rm -rf -- "$TARGET"
        exit 1
    fi
    source_label="remote: $REMOTE_BASE"
fi

printf 'Installed "%s" -> %s\n' "$SKILL_NAME" "$TARGET"
printf '  source: %s\n' "$source_label"

if ! command -v sel >/dev/null 2>&1; then
    printf '\nNote: `sel` is not on your PATH. The skill only activates when\n'
    printf '      `sel` is actually available. Install it with:\n'
    printf '        cargo install sel-rs\n'
fi
