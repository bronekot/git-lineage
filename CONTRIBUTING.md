# Contributing

Thank you for helping improve `git-lineage`.

## Before opening an issue

- Search existing issues first.
- Remove tokens, credential-bearing URLs, private repository names, and other secrets from logs.
- Include the operating system, `git --version`, `git lineage --version`, the command used, and a minimal sanitized `.git-lineage.toml` when relevant.

Security vulnerabilities must not be reported in public issues. Follow [SECURITY.md](SECURITY.md).

## Development setup

Requirements:

- Rust 1.85 or newer
- Git

```console
git clone https://github.com/bronekot/git-lineage
cd git-lineage
cargo test --all-targets
```

Before submitting a pull request, run:

```console
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo package --locked
```

## Pull requests

- Keep changes focused and explain observable behavior changes.
- Add regression tests for fixes and integration tests for Git-state behavior.
- Update `README.md`, `README.ru.md`, and `CHANGELOG.md` when user-facing behavior changes.
- Do not add shell-interpolated Git execution or operations that mutate local branches, commits, or the working tree.
- Keep stdout valid JSON in JSON mode; diagnostics belong on stderr.

By contributing, you agree that your contribution is licensed under the project's MIT License.
