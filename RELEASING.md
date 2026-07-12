# Releasing

This document is for project maintainers.

## Prerequisites

- Write access to `bronekot/git-lineage`.
- Ownership of the `git-lineage` crate on crates.io.
- A clean `main` branch with passing CI.
- `cargo login` configured locally, or an equivalent crates.io trusted-publishing setup.

## Prepare a release

1. Update `version` in `Cargo.toml` and run `cargo check` so `Cargo.lock` is current.
2. Move entries from `Unreleased` to a dated version in `CHANGELOG.md`.
3. Run the complete verification:

   ```console
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all-targets --locked
   cargo package --locked
   cargo publish --dry-run --locked
   ```

4. Commit the release preparation and merge it into `main`.
5. Create and push an annotated tag matching the Cargo version:

   ```console
   git tag -a v0.1.0 -m "git-lineage 0.1.0"
   git push origin v0.1.0
   ```

The release workflow verifies that the tag matches `Cargo.toml`, builds Windows, Linux musl, and Apple Silicon macOS archives, and publishes a GitHub release.

## Publish to crates.io

After the tagged GitHub build succeeds:

```console
cargo publish --locked
```

Verify installation from the registry in a clean environment:

```console
cargo install git-lineage --version 0.1.0
git lineage --version
```

Do not reuse or move a published version tag. If publication fails after a GitHub release, fix the cause and publish a new patch version when required by crates.io immutability rules.
