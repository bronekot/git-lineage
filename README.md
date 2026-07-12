# git-lineage

[![CI](https://github.com/bronekot/git-lineage/actions/workflows/ci.yml/badge.svg)](https://github.com/bronekot/git-lineage/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

English · [Русский](README.ru.md)

`git-lineage` is a cross-platform Rust CLI that renders a compact semantic tree of Git branches. It focuses on relationships between branches instead of printing every commit: logical parents, ahead/behind counts, tracking divergence, sibling comparisons, and synced remote branches.

```text
Fetched origin; branch graph:

main
├─ dev (current)                 +8 from main
│  └─ origin/dev                 +1 from dev
│     └─ commands-fun            +12 from origin/dev
└─ web-ui                        +13 from main
```

## Installation

From the Git repository:

```console
cargo install --git https://github.com/bronekot/git-lineage
```

After the first crates.io release:

```console
cargo install git-lineage
```

Cargo installs the `git-lineage` executable. Git automatically exposes executables named `git-*` as external subcommands:

```console
git lineage
```

Prebuilt archives for Windows, Linux, and macOS are attached to tagged GitHub releases.

## Usage

By default, `git lineage` runs `git fetch --prune --quiet origin` before building the report. To use only locally cached refs:

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
| `git lineage discover` | Print conservatively inferred parents |
| `git lineage discover --write` | Save inferred parents |
| `git lineage parent <CHILD> <PARENT>` | Set a logical parent |
| `git lineage parent <CHILD> --unset` | Remove a logical parent |
| `git lineage check` | Validate refs, ancestry, remotes, and cycles |
| `git lineage language [en|ru]` | Show or persist the interface language |

Useful report options:

```console
git lineage --summary none|brief|full
git lineage --format text|json
git lineage --color auto|always|never
git lineage --ascii
git lineage --all-remotes
git lineage --show-synced-remotes
```

## Language

English is the default interface language. Show the current language or persist a different one with:

```console
git lineage language
git lineage language ru
git lineage language en
```

The selection is stored in the global Git configuration as `git-lineage.language`, so it applies to every repository and survives terminal restarts. The `lang` alias is also accepted. JSON field names and insight kinds remain language-neutral; human-readable warnings follow the selected language.

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

Use full ref names such as `refs/heads/dev` when a short name is ambiguous.

## Safety

`git-lineage` invokes the system Git directly with separate process arguments and sets `GIT_TERMINAL_PROMPT=0`. It never runs a shell and never performs `merge`, `rebase`, `reset`, `cherry-pick`, or `push`. The optional fetch only updates remote-tracking refs.

If fetch fails but cached remote-tracking refs are available, the report continues and clearly identifies that it used cached data. JSON mode keeps `stdout` machine-readable and sends diagnostics to `stderr`.

## Platform and toolchain support

- Windows, Linux, and macOS
- Rust 1.85 or newer
- System Git available on `PATH`

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md). Release maintainers should also read [RELEASING.md](RELEASING.md). Security issues must follow [SECURITY.md](SECURITY.md).

## License

Licensed under the [MIT License](LICENSE).
