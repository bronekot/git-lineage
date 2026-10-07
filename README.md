# git-lineage

[![CI](https://github.com/bronekot/git-lineage/actions/workflows/ci.yml/badge.svg)](https://github.com/bronekot/git-lineage/actions/workflows/ci.yml) [![crates.io](https://img.shields.io/crates/v/git-lineage.svg)](https://crates.io/crates/git-lineage) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

English · [Русский](README.ru.md)

`git-lineage` renders the factual history-inclusion graph of every local and remote branch ref. It uses only Git history—there is no project configuration and no concept of an assigned or logical parent.

The graph is built as follows:

1. Read every ref under `refs/heads` and `refs/remotes`.
2. Group names that point to the same commit.
3. Compute the left-only and right-only commit counts for every pair, equivalent to `git rev-list --left-right --count A...B`.
4. Order histories by strict inclusion and remove transitive edges.
5. Report diverged pairs separately. A merge ref can therefore have multiple immediate history parents.

```text
Factual ref graph from local data:

main (= origin/main)
└─ dev (= origin/dev, origin/player-fix)  +8 from main
   └─ commands-fun (current)              +12 from dev

Refs: 6, nodes: 3 (coincident refs merged: 3), edges after transitive reduction: 2.
```

## Installation

Install the latest release from crates.io:

```console
cargo install git-lineage --locked
```

Cargo normally installs the executable into `~/.cargo/bin`, or `%USERPROFILE%\.cargo\bin` on Windows. That directory must be in `PATH` for Git to discover the external `git-lineage` command.

```console
git-lineage --version
git lineage --version
```

Install the current 0.2 development branch with:

```console
cargo install --git https://github.com/bronekot/git-lineage --branch dev --locked
```

[GitHub Releases](https://github.com/bronekot/git-lineage/releases) provide archives for Windows x86_64, Linux x86_64, and macOS Apple Silicon.

## Usage

By default, `git lineage` fetches and prunes `origin` before reading all locally stored branch refs. Select another remote to fetch with `--remote`; refs from every remote are always included in the graph.

```console
git lineage
git lineage --no-fetch
git lineage --offline
git lineage --remote upstream
```

| Command | Purpose |
| --- | --- |
| `git lineage` | Build the factual graph |
| `git lineage report` | Build the factual graph explicitly |
| `git lineage language [LANGUAGE]` | Show or save the interface language (`en` or `ru`) |

| Option | Effect |
| --- | --- |
| `--summary none` | Omit explanatory insights; warnings remain visible |
| `--summary brief` | Explain tracking relations, deleted upstreams, and how the current branch diverged from its siblings (default) |
| `--summary full` | Explain every divergent ref pair |
| `--format json` | Include nodes, all pair relations, reduced edges, insights, and counters as JSON |
| `--color always` | Force color; `auto` (honors `NO_COLOR`) and `never` are also supported |
| `--ascii` | Use ASCII tree characters |
| `--remote NAME` | Fetch `NAME` instead of `origin`; this does not filter displayed refs |

In brief mode the number of other diverged pairs is shown as a hint and as `hidden_diverged_pairs` in JSON.

Each JSON pair relation contains `left_only` and `right_only` counts plus one of `left_ancestor`, `right_ancestor`, or `diverged`. Equal commit pointers are represented as aliases in one node.

## Language

English is the default. The selection is stored globally as `git-lineage.language` and survives terminal restarts:

```console
git lineage language
git lineage language ru
git lineage language en
```

JSON field names and insight `kind` values do not depend on the selected language.

## Configuration

There is no `.git-lineage.toml`. Files with that name are ignored. The report is derived entirely from refs and commit history; `init`, `discover`, `parent`, and `check` are not 0.2 commands.

## Safety

`git-lineage` invokes system Git with separate process arguments and sets `GIT_TERMINAL_PROMPT=0`. It never asks Git to perform `merge`, `rebase`, `reset`, `cherry-pick`, or `push`.

The default report does run `git fetch --prune`. Git may update or remove remote-tracking refs, write `FETCH_HEAD`, follow tags, fetch configured submodules, and run maintenance. Use `--no-fetch` to avoid network access and ref changes. If the remote is not configured, the report uses local data without an error. If fetching fails, the report continues with cached refs when possible. JSON diagnostics are written to `stderr` so `stdout` remains valid JSON.

## Platform and toolchain support

- Windows, Linux, and macOS
- Rust 1.85 or newer
- System Git available in `PATH`

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md), [Code of Conduct](CODE_OF_CONDUCT.md), [RELEASING.md](RELEASING.md), and [SECURITY.md](SECURITY.md).

## License

Licensed under the [MIT License](LICENSE).
