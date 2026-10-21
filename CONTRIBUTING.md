# Contributing

## Local checks

No hosted CI is used by this project. Run the gate locally before opening a PR:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
./target/release/readme-gen check --strict
```

On Windows (PowerShell):

```powershell
.\scripts\check.ps1
```

`scripts/check.sh` runs the same steps and skips `fmt`/`clippy` when those
components are not installed.

## Rules of the road

1. **Facts are not hand-edited.** If a count or size changes, rerun
   `readme-gen build`; do not patch the generated number.
2. **Prose belongs in `readme.yml`.** If you need a paragraph that the
   generator cannot produce, add it under `sections:` or use a `keep:` block.
3. **New sections need a preset entry and a test.** A renderer that is not in
   `outline.rs` never runs.
4. **New numeric output must be traceable.** Add it to `Facts` (and its JSON)
   so the checker can approve it; otherwise `check --strict` fails.
5. **Keep it dependency-free.** The crate builds with the standard library
   only; adding a crate needs a strong justification and a cross-platform test.
6. **Model choices are documentation, not code.** Recommended models, quants and
   launch flags live in [docs/MODELS.md](docs/MODELS.md); do not hardcode
   model-specific prompts in the renderer.

## Adding a language

`src/scan.rs::language_for` maps extensions to display names, and
`src/lang.rs` defines locale ids. Both are small and covered by tests.

## Reporting a bug

Include:

- the output of `readme-gen scan --json` (or a minimal fixture);
- the `readme.yml` you used;
- the exact `readme-gen` version (`readme-gen --version`) and platform.
