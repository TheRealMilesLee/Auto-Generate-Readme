# Benchmarks

Numbers below are measured, not estimated. Re-run the commands on your own
machine; filesystem cache and disk type dominate the cold scan.

## Environment

| Item | Value |
|---|---|
| Machine | Apple M4 |
| OS | macOS 27.2 |
| Toolchain | rustc 1.99.1, `--release` (LTO, `codegen-units = 1`, `panic = "abort"`) |
| Binary | `target/release/readme-gen`, 922 KiB |

## Scan

Target: `~/Pictures/The-Wallpaper-Collection` — a media repository with
**2,717 files**, **2,685 images**, **18 videos**, **109 directories**, **30
top-level folders**, **5.8 GiB** of media (state on 2026-10-21).

```bash
/usr/bin/time -p ./target/release/readme-gen scan \
  --root ~/Pictures/The-Wallpaper-Collection \
  --out /tmp/facts.json
```

| Run | Real | User | Sys | Max RSS |
|---|---|---|---|---|
| Cold (first run) | 1.00 s | 0.05 s | 0.04 s | — |
| Warm | 0.10 s | 0.05 s | 0.03 s | 6.9 MB |

The scanner calls `symlink_metadata` once per entry and never reads file
contents (except manifests, licence and `.gitignore`). It does not shell out to
`find`, `tree` or `du`. That is why a 5.8 GiB tree is bounded by directory
metadata, not by media size.

## Build

Generate this repository's own README (`preset: cli`, two locales) three times:

```bash
for i in 1 2 3; do
  /usr/bin/time -p ./target/release/readme-gen build --root . --out /tmp/README.md
done
```

| Run | Real |
|---|---|
| 1 | 0.08 s |
| 2 | 0.06 s |
| 3 | 0.06 s |

Output: 11.7 KiB, 18 sections across two languages, byte-identical between runs
when facts and `readme.yml` are unchanged.

## Check

```bash
for i in 1 2 3; do
  /usr/bin/time -p ./target/release/readme-gen check --root . --out README.md
done
```

| Run | Real |
|---|---|
| 1 | 0.05 s |
| 2 | 0.05 s |
| 3 | 0.05 s |

`check` re-scans the repository (same cost as scan), parses the Markdown, walks
every link, and traces every number.

## LLM prose

Not benchmarked here because it depends on the model and hardware. The design
goal is that the model is **not** on the hot path:

- `llama-server` keeps the model and KV cache loaded;
- only prose sections (`why`, `audience`) call the model;
- responses are cached by facts hash + prompt version + model, so a rebuild
  with unchanged facts performs zero requests;
- deterministic builds never touch the network.

On an 8B Q4 model, expect seconds per section rather than per document.

## Reading the numbers

- A cold scan pays for the first read of directory metadata; a warm scan does
  not. Both are far below the time to hash or copy the media itself.
- The binary is small because it has no dependencies and no async runtime.
- These numbers are a floor, not a promise: a 500k-file repository will take
  longer, which is why `--max-files` exists as a safety valve.
