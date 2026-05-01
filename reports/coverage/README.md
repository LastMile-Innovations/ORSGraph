# Coverage Reports

Coverage outputs in this directory are generated artifacts and are ignored by
Git. Generate a current Rust LCOV report with:

```sh
cargo llvm-cov --workspace --lcov --output-path reports/coverage/rust-lcov.info
```

For a quick terminal summary, run:

```sh
cargo llvm-cov --workspace --summary-only
```

Frontend coverage lives under `frontend/coverage/` and is generated from the
frontend workspace:

```sh
pnpm run coverage
pnpm run coverage:all
```
