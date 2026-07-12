# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- English is now the default interface language.
- Added persistent `git lineage language en|ru` switching through global Git configuration.
- Localized CLI help, reports, insights, warnings, and structured errors in English and Russian.

## [0.1.0] - 2026-07-12

### Added

- Semantic branch trees driven by configuration and conservative discovery.
- Automatic tracking relationships with synced-upstream collapsing.
- Ahead/behind, divergence, root-to-leaf, and sibling insights.
- Text, JSON, ASCII, color, brief, and full report modes.
- `init`, `discover`, `parent`, and `check` commands.
- Safe fetch fallback, detached HEAD detection, and shallow-clone warnings.
- Windows, Linux, and macOS support with bounded Git-process concurrency.

[Unreleased]: https://github.com/bronekot/git-lineage/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/bronekot/git-lineage/releases/tag/v0.1.0
