# Cargo.lock Required

## Critical: Generate Cargo.lock

This file is required for deterministic builds in Docker/Railway.

### To generate:

```bash
cargo build
# або
cargo check
```

This will create `Cargo.lock` with pinned dependency versions.

### Add to git:

```bash
git add Cargo.lock
git commit -m "Add Cargo.lock for deterministic builds"
```

**DO NOT add Cargo.lock to .gitignore for binary crates!**

Only library crates should exclude Cargo.lock.
