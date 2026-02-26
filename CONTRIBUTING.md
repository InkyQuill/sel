# Contributing to Sel

Thank you for your interest in contributing to Sel!

## Development

```bash
# Clone the repository
git clone https://github.com/InkyQuill/sel.git
cd sel

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --check

# Build release
cargo build --release
```

## Commit Convention

This project follows [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — New feature
- `fix:` — Bug fix
- `docs:` — Documentation changes
- `test:` — Test changes
- `refactor:` — Code refactoring
- `chore:` — Maintenance tasks

Examples:
- `feat: add support for stdin input`
- `fix: handle empty files correctly`
- `docs: update installation instructions`

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
