# Benchmarks

Deterministic numbers below are measured, not estimated. Re-run the commands on
your own machine; filesystem cache and disk type dominate the cold scan. The LLM
table is explicitly labelled as an estimate, because model latency depends on
hardware, TGP and thermals.

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

The numbers above are deterministic and measured. Model latency is not: it
depends on the GPU, TGP, thermals, context and quantisation, so the table below
is an **estimate** for the project's target laptop, not a measurement.

### Target laptop

| Item | Value |
|---|---|
| CPU | Ryzen 7 7735HS (8C/16T, Zen 3+) |
| RAM | 32 GB DDR5-4800 (dual channel, ~76.8 GB/s theoretical) |
| GPU | RTX 4070 Laptop, 8 GB GDDR6 |
| Storage | 3 TB SSD |
| OS | Windows (Git Bash / PowerShell) or Linux |

8 GB VRAM is the hard constraint; 32 GB RAM is what makes 14B and 30B-A3B
partial offload practical. Close GPU-heavy applications before serving.

### Estimated generation speed

| Model | Quant | Placement | Generation | 400-token section |
|---|---|---|---|---|
| Qwen3-4B-Instruct-2507 | `Q5_K_M` | full GPU | 60–90 tok/s | ~5 s |
| Qwen2.5-7B / Qwen3-8B | `Q5_K_M` | full GPU | 35–55 tok/s | ~8–12 s |
| Qwen3-14B | `Q4_K_M` | partial (GPU + CPU) | 12–20 tok/s | ~20–35 s |
| Qwen3-30B-A3B | `Q4_K_M` | partial (MoE) | 10–18 tok/s | ~25–40 s |

A build writes at most two prose sections, and responses are cached by facts
hash + prompt version + model. The first run costs seconds to tens of seconds;
a rebuild with unchanged facts performs zero requests. Deterministic builds
never touch the network.

These figures assume CUDA, 8k context, the model already loaded, and a laptop
TGP around 100 W. Sustained loads throttle; battery mode is slower. Do not use
them as a model benchmark — use them to choose a starting point, then measure
on your own machine.

Model-specific launch flags and alternatives: [MODELS.md](MODELS.md).

## Reading the numbers

- A cold scan pays for the first read of directory metadata; a warm scan does
  not. Both are far below the time to hash or copy the media itself.
- The binary is small because it has no dependencies and no async runtime.
- These numbers are a floor, not a promise: a 500k-file repository will take
  longer, which is why `--max-files` exists as a safety valve.
