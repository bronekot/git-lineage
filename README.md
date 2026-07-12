# git-lineage

[![CI](https://github.com/bronekot/git-lineage/actions/workflows/ci.yml/badge.svg)](https://github.com/bronekot/git-lineage/actions/workflows/ci.yml) [![crates.io](https://img.shields.io/crates/v/git-lineage.svg)](https://crates.io/crates/git-lineage) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

English · [Русский](README.ru.md)

`git-lineage` is a cross-platform Rust CLI that renders a compact semantic tree of Git branches. It focuses on relationships instead of printing every commit: logical parents, ahead/behind counts, upstream divergence, sibling comparisons, and synced upstream branches.

With explanatory insights disabled, `git lineage --summary none` produces output like this:

```text
Fetched origin; branch graph:

main
├─ dev (current)                 +8 from main
│  └─ origin/dev                 +1 from dev
│     └─ commands-fun            +12 from origin/dev
└─ web-ui                        +13 from main
```

## Installation

Recommended installation from crates.io; `--locked` uses the dependency versions tested for the release:

```console
cargo install git-lineage --locked
```

Cargo installs `git-lineage` into the configured installation root's `bin` directory. With the default settings, this is `$CARGO_HOME/bin`—normally `~/.cargo/bin`, or `%USERPROFILE%\.cargo\bin` on Windows. That directory must be present in `PATH`; Git only discovers external `git-*` commands that it can find in `PATH`.

Verify both executable forms after installation:

```console
git-lineage --version
git lineage --version
```

If `git-lineage` works but `git lineage` does not, check that the directory containing `git-lineage` is in `PATH` and restart the terminal.

To install the current development version from `main` instead of the latest crates.io release:

```console
cargo install --git https://github.com/bronekot/git-lineage --locked
```

[GitHub Releases](https://github.com/bronekot/git-lineage/releases) provide prebuilt archives for these targets:

- Windows x86_64: `x86_64-pc-windows-msvc`
- Linux x86_64: `x86_64-unknown-linux-musl`
- macOS Apple Silicon: `aarch64-apple-darwin`

After extracting an archive, move the `git-lineage` executable—or its containing directory—to a location in `PATH`.

## Usage

With the default settings, `git lineage` runs `git fetch --prune --quiet -- origin` before building the report. To use the refs already stored in the local repository:

```console
git lineage --no-fetch
git lineage --offline
```

The report header indicates whether the remote was fetched, fetching was skipped, or cached data was used after a failed fetch.

Common commands:

| Command | Purpose |
| --- | --- |
| `git lineage` | Build the default report |
| `git lineage init` | Create `.git-lineage.toml` |
| `git lineage discover` | Print a complete proposed TOML configuration |
| `git lineage discover --write` | Rewrite `.git-lineage.toml` with discovered relationships |
| `git lineage parent <CHILD> <PARENT>` | Set a logical parent |
| `git lineage parent <CHILD> --unset` | Remove a logical parent |
| `git lineage check` | Validate refs, configured ancestry, the selected remote, and cycles |
| `git lineage language [LANGUAGE]` | Show or save the interface language; supported values are `en` and `ru` |

`discover --write` and `parent` serialize the entire TOML file. Even when only relationships change, comments and hand formatting are lost. Back up or commit `.git-lineage.toml` before using them on a file whose formatting matters. `init --force` overwrites an existing configuration file.

Report options:

| Option | Effect |
| --- | --- |
| `--summary none` | Omit explanatory insights; warnings are still shown |
| `--summary brief` | Show the default concise insights |
| `--summary full` | Show all insights, inferred parents, remote-only refs, and unplaced branches |
| `--format json` | Emit machine-readable JSON instead of text |
| `--color always` | Force color; `auto` and `never` are also supported |
| `--ascii` | Use ASCII tree characters |
| `--all-remotes` | Allow refs from every remote into the report's scope |
| `--show-synced-remotes` | Show synced upstream refs instead of collapsing them |

`--all-remotes` does not by itself display every remote-only ref. Combine it with `--summary full`, or set `show_remote_only = true`, when those refs should appear.

## Language

English is the default interface language. Show the current language or save a different one with:

```console
git lineage language
git lineage language ru
git lineage language en
```

The selection is stored in the global Git configuration as `git-lineage.language`, so it applies to every repository and survives terminal restarts. The `lang` alias is also accepted. JSON field names and insight `kind` values remain stable across languages. Interface text produced directly by `git-lineage` follows the selected language; diagnostics from Git, the operating system, or parsing libraries may retain their own language.

## Configuration

`.git-lineage.toml` is optional. Explicit parent relationships take priority over automatic discovery.

```toml
version = 1
root = "main"
remote = "origin"
fetch = true

[parents]
dev = "main"
feature = "origin/dev"

[display]
collapse_synced_upstreams = true
show_remote_only = false
summary = "brief"
```

- `collapse_synced_upstreams` hides an upstream ref when it points to the same commit as its local branch.
- `show_remote_only` includes remote refs that are not an upstream or an explicitly configured branch.
- `summary` accepts `none`, `brief`, or `full`.

Use full ref names such as `refs/heads/dev` when a short name is ambiguous.

## Safety and repository changes

`git-lineage` invokes the system Git with separate process arguments instead of constructing a shell command, and sets `GIT_TERMINAL_PROMPT=0`. It never asks Git to perform `merge`, `rebase`, `reset`, `cherry-pick`, or `push`. The system Git may still invoke credential, transport, or SSH helpers from the user's Git configuration.

The default report runs `git fetch --prune`. Its exact effects are governed by Git configuration and the selected remote's refspecs: Git writes `FETCH_HEAD`, updates or prunes configured refs, may auto-follow tags, may fetch populated submodules, and may run automatic maintenance. Use `--no-fetch` to prevent this network operation and its ref updates.

The report command does not write `.git-lineage.toml`. The `init`, `discover --write`, and `parent` commands do write it, while `language` writes `git-lineage.language` to the global Git configuration.

If fetch fails and the refs already stored locally are sufficient, the report continues and identifies that it used cached data. In JSON mode, `stdout` remains machine-readable and diagnostics are sent to `stderr`.

## Platform and toolchain support

- Windows, Linux, and macOS when built with Cargo
- Rust 1.85 or newer
- System Git available in `PATH`
- `git-lineage` available in `PATH` for the `git lineage` form

See the exact prebuilt targets in the [Installation](#installation) section.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md). Release maintainers should also read [RELEASING.md](RELEASING.md). Security issues must follow [SECURITY.md](SECURITY.md).

## License

Licensed under the [MIT License](LICENSE).
