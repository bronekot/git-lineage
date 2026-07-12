# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
