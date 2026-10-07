# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Replaced the configured semantic tree with a factual history-inclusion DAG for every local and remote branch ref.
- Compute pair relations from left-only and right-only commit sets, equivalent to `git rev-list --left-right --count A...B`.
- Apply a transitive reduction to strict history inclusion, preserving multiple immediate parents for merge refs.
- Group refs that point to the same commit into one node with visible aliases.
- Removed `.git-lineage.toml`, `init`, `discover`, `parent`, `check`, and all logical-parent behavior.
- Brief summaries describe only the current branch's divergence from its siblings and count the remaining diverged pairs; `--summary full` lists every pair.
- Compute all pair counts from one `git rev-list --topo-order --stdin` pass with per-commit reachability bitsets, keeping memory and time bounded for large repositories and avoiding command-line length limits.
- Resolve the current branch by its full ref name and skip symbolic remote refs by type instead of by name.
- The text report ends with a single refs/nodes/edges line; JSON `refs.shown` was replaced by `refs.nodes`.

### Added

- Report completeness and reduced-edge counters.
- Emit every pair relation and every reduced parent edge in JSON.
- Describe diverged histories separately; brief mode covers immediate siblings and full mode covers every diverged pair.
- Report local branches whose upstream ref no longer exists.
- Treat an unconfigured fetch remote as local-only data instead of a fetch failure.
- Honor `NO_COLOR` in automatic color mode.
- Characterization coverage for the full `xdub` topology, coincident refs, divergence, merge DAGs, unrelated roots, and 100-ref repositories.

## [0.1.1] - 2026-07-13

### Fixed

- Corrected crates.io, development, prebuilt archive, and `PATH` installation instructions.
- Repaired command and option tables whose `en|ru` notation broke Markdown columns.
- Clarified report examples, discovery output, summary modes, remote scope, configuration rewrites, localization boundaries, and the effects of `git fetch`.

## [0.1.0] - 2026-07-12

### Added

- Semantic branch trees driven by configuration and conservative discovery.
- Automatic tracking relationships with synced-upstream collapsing.
- Ahead/behind, divergence, root-to-leaf, and sibling insights.
- Text, JSON, ASCII, color, brief, and full report modes.
- `init`, `discover`, `parent`, and `check` commands.
- English and Russian interfaces with English as the default and persistent `git lineage language en|ru` switching.
- Localized CLI help, reports, insights, warnings, and structured errors.
- Safe fetch fallback, detached HEAD detection, and shallow-clone warnings.
- Windows, Linux, and macOS support with bounded Git-process concurrency.

[Unreleased]: https://github.com/bronekot/git-lineage/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/bronekot/git-lineage/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/bronekot/git-lineage/releases/tag/v0.1.0
