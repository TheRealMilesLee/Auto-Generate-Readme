# Local model guide

`readme-gen` uses a model for **two short prose sections** (`why`, `audience`).
It does not summarise the repository, write code, or reason about architecture.
That changes the optimisation target: instruction following and writing quality
matter; raw model size does not.

## TL;DR for this machine

Hardware: **Ryzen 7 7735HS · 32 GB DDR5-4800 · RTX 4070 Laptop 8 GB · 3 TB SSD**.

| Pick | Model | Quant | Why |
|---|---|---|---|
| **Default** | Qwen2.5-7B-Instruct | `Q5_K_M` | no thinking overhead, strong English + Chinese, ~5.4 GB, fits fully in 8 GB VRAM |
| **Newer default** | Qwen3-8B | `Q5_K_M` | better instruction following, ~5.8 GB; **must disable thinking** |
| **English upgrade** | Gemma 3 12B IT | `Q4_K_M` | best prose in this class, ~7.3 GB, borderline for 8 GB; Chinese is weaker |
| **Quality upgrade** | Qwen3-14B | `Q4_K_M` | ~9 GB, partial CPU offload, noticeably better consistency |
| **Max quality** | Qwen3-30B-A3B | `Q4_K_M` | ~18.6 GB MoE (3B active), fits in 32 GB RAM with partial GPU offload; slower |
| **Fast** | Qwen3-4B-Instruct-2507 | `Q5_K_M` | ~3 GB, 2–3× faster, good enough for two paragraphs |

**Do not** use reasoning models (QwQ, R1 distills, thinking-mode Qwen3) for this
job. They spend hundreds of tokens thinking, often leak the reasoning into the
answer, and the validator will reject it. If you must use Qwen3, disable
thinking at the server (below).

The real quality lever is [`readme.yml`](CONFIGURATION.md): a good tagline and
two concrete folder annotations beat a 30B model with empty annotations.

## Why 8 GB VRAM is the constraint

Approximate budget on a laptop with the display attached:

```
8.0 GB physical
− 0.5–1.5 GB desktop / browser / driver
≈ 6.5–7.5 GB usable by CUDA
− KV cache (8k context, 8B model: ~0.5 GB)
− compute buffers / fragmentation (~0.5 GB)
≈ 5.5–6.5 GB for weights
```

That is why an 8B `Q5_K_M` (~5.8 GB) fits, a 12B `Q4_K_M` (~7.3 GB) is
borderline, and a 14B `Q4_K_M` (~9 GB) must spill a few layers to CPU. Close
browsers and GPU-accelerated apps before a run; on Windows, check
`nvidia-smi` while the server is loaded.

The 32 GB of system RAM is the useful part for larger models: it lets a 14B or
a 30B-A3B MoE keep the non-offloaded weights in memory instead of swapping.
DDR5-4800 dual-channel is roughly 76.8 GB/s theoretical; expect materially less
in practice.

## Recommendations in detail

### Qwen2.5-7B-Instruct — the safe default

- 7.6B parameters, Apache-2.0, no thinking mode.
- `Q5_K_M` ≈ 5.4 GB; fully on GPU with 8k context.
- Strong Chinese and English, reliable instruction following, predictable.
- Best choice if you want `readme-gen build --llm` to "just work" without
  server-side flags.

### Qwen3-8B — newer, slightly better, needs one flag

- 8.2B parameters, Apache-2.0.
- `Q5_K_M` ≈ 5.8 GB; still fits fully.
- Defaults to thinking. On `llama-server`, disable it:

  ```bash
  llama-server ... --jinja --reasoning-budget 0
  ```

  or, if your build supports chat-template kwargs:

  ```bash
  llama-server ... --jinja --chat-template-kwargs '{"enable_thinking":false}'
  ```

  With Ollama, prefer `qwen2.5:7b-instruct` unless you can pass `think: false`
  through your gateway; the `readme-gen` client does not set it.

### Gemma 3 12B IT — English prose upgrade

- `Q4_K_M` ≈ 7.3 GB. At 8k context it exceeds 8 GB; use `-c 4096` and close
  other GPU apps, or accept a small CPU spill.
- Excellent English style; Chinese is usable but behind Qwen.
- Gemma Terms of Use, not an OSI licence — check before redistribution.

### Qwen3-14B — the quality/speed knee

- `Q4_K_M` ≈ 9 GB. Keep ~7 GB of layers on GPU and the rest on CPU.
- Better at following a longer annotation set and avoiding repetition than 7–8B.
- Expect roughly half the speed of an 8B model.

### Qwen3-30B-A3B — maximum quality on 32 GB

- `Q4_K_M` ≈ 18.6 GB; 30B total, ~3B active per token (MoE).
- Put attention on the GPU and experts on the CPU with `--n-cpu-moe` (see
  `llama-server --help`; the flag name has changed across builds).
- This is the best bilingual prose available on this laptop, but it is 2–3×
  slower than 8B and leaves little RAM headroom. Close everything else.
- Use it when you are polishing a showcase README, not for CI.

### Qwen3-4B-Instruct-2507 — fast lane

- `Q5_K_M` ≈ 3 GB, fully on GPU, very fast.
- Non-thinking by design.
- Good for iterating on `readme.yml`; final polish can use a larger model.

### English-only alternatives

- **Mistral Nemo 12B Instruct** `Q4_K_M` ≈ 7.1 GB, Apache-2.0, strong English.
- **gpt-oss-20b** MXFP4 ≈ 12.1 GB, MoE ~3.6B active; with 8 GB VRAM + 32 GB RAM
  it runs via CPU offload (`--n-cpu-moe`). English-centric; Chinese is weaker.
- If the README is English-only, these are reasonable; for this bilingual
  project, Qwen is the better fit.

## Quantization

| Quant | Use it for | Notes |
|---|---|---|
| `Q5_K_M` | 7–8B | best quality that still fits fully in 8 GB |
| `Q4_K_M` | 12B–30B | the standard quality/size trade-off |
| `Q5_K_M` / `Q6_K` | 4B | cheap quality |
| `Q3_K_M` and below | avoid | prose gets repetitive and fact-unsafe |

Prefer quants from a known uploader (bartowski, unsloth, model author) and keep
the model card's chat template. File naming varies; the number after `Q` is the
quant, not the parameter count.

## llama.cpp setup

### Windows (CUDA, Git Bash or PowerShell)

1. Install the NVIDIA driver and a CUDA build of llama.cpp. Prebuilt
   `llama-server.exe` binaries are published on the
   [llama.cpp releases page](https://github.com/ggml-org/llama.cpp/releases).
2. Download a GGUF, e.g. `Qwen2.5-7B-Instruct-Q5_K_M.gguf`.
3. Serve it:

   ```powershell
   .\llama-server.exe -m .\models\Qwen2.5-7B-Instruct-Q5_K_M.gguf `
     --host 127.0.0.1 --port 8080 `
     -ngl 99 -c 8192 -fa on --jinja
   ```

4. Verify the startup log says the layers were offloaded to CUDA, then:

   ```bash
   readme-gen build --llm http://127.0.0.1:8080 --model qwen2.5-7b
   ```

### Linux (CUDA)

```bash
cmake -B build -DGGML_CUDA=ON -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release -j"$(nproc)"

./build/bin/llama-server \
  -m models/Qwen2.5-7B-Instruct-Q5_K_M.gguf \
  --host 127.0.0.1 --port 8080 \
  -ngl 99 -c 8192 -fa on --jinja
```

### Useful flags

| Flag | Meaning |
|---|---|
| `-ngl 99` | offload all transformer layers to GPU |
| `-ngl N` | offload only N layers (partial offload) |
| `-c 8192` | context size; 8k is plenty for this tool |
| `-fa on` | flash attention (lower KV memory) |
| `--jinja` | use the model's chat template |
| `--parallel 1` | one slot; avoids multiplying KV cache |
| `--reasoning-budget 0` | disable thinking (recent builds) |
| `--n-cpu-moe N` | keep MoE experts on CPU (30B-A3B / gpt-oss) |

Do not set `-c 32768` "just in case": KV cache grows linearly and can push an
otherwise-fitting model out of VRAM.

## Ollama alternative

```bash
ollama pull qwen2.5:7b-instruct
OLLAMA_KEEP_ALIVE=30m OLLAMA_NUM_PARALLEL=1 ollama serve

readme-gen build --llm ollama --model qwen2.5:7b-instruct
```

`ollama ps` shows whether the model is 100% GPU. `OLLAMA_KEEP_ALIVE` keeps it
resident between builds; otherwise the first section pays the load time.

Ollama's OpenAI-compatible endpoint is `http://127.0.0.1:11434`; `readme-gen`
uses it automatically with `--llm ollama`.

## Expected performance (estimates)

These are rough ranges for a 4070 Laptop (TGP and thermals vary), 8k context,
CUDA, with the model already loaded:

| Model | Quant | Placement | Generation | 400-token section |
|---|---|---|---|---|
| Qwen3-4B-2507 | Q5_K_M | full GPU | 60–90 tok/s | ~5 s |
| Qwen2.5-7B / Qwen3-8B | Q5_K_M | full GPU | 35–55 tok/s | ~8–12 s |
| Qwen3-14B | Q4_K_M | partial | 12–20 tok/s | ~20–35 s |
| Qwen3-30B-A3B | Q4_K_M | partial (MoE) | 10–18 tok/s | ~25–40 s |

The tool writes at most two sections per build, and responses are cached. A
first run costs seconds to tens of seconds; later runs with unchanged facts
cost nothing. Do not benchmark a model by loading the document from scratch
each time.

## How to choose without guessing

1. Start with Qwen2.5-7B-Instruct `Q5_K_M`; run `readme-gen build` with both
   locales.
2. If a section is rejected, the warning names the invented number. Fix the
   annotation or try a larger model; do not "fix" it by loosening `check`.
3. If the prose is bland, the problem is usually `readme.yml`, not the model.
   Add one concrete sentence per folder and a sharper tagline.
4. Compare two models with the same facts and annotations:

   ```bash
   readme-gen build --out /tmp/readme-7b.md --llm http://127.0.0.1:8080 --model qwen2.5-7b
   readme-gen build --out /tmp/readme-14b.md --llm http://127.0.0.1:8081 --model qwen3-14b
   diff -u /tmp/readme-7b.md /tmp/readme-14b.md
   ```

   Keep the version whose prose survives `check --strict` and needs fewer edits.

## Privacy and licensing

- The client sends facts (counts, sizes, folder names, annotations), never file
  contents.
- The endpoint must be `http://`; run the model locally. A remote endpoint is
  not supported without a local reverse proxy.
- Model licences vary: Qwen2.5/Qwen3 are Apache-2.0, Gemma uses the Gemma Terms
  of Use, Mistral Nemo and gpt-oss are Apache-2.0. Check the model card before
  redistributing weights or generated text in a commercial context.
- The README's generated prose is your content; the model's licence does not
  change the repository's licence, but do not ship model weights in this repo.

## Model sources

- [Qwen2.5-7B-Instruct](https://huggingface.co/Qwen/Qwen2.5-7B-Instruct)
- [Qwen3-8B](https://huggingface.co/Qwen/Qwen3-8B) ·
  [Qwen3-4B-Instruct-2507](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507)
- [Qwen3-14B](https://huggingface.co/Qwen/Qwen3-14B) ·
  [Qwen3-30B-A3B](https://huggingface.co/Qwen/Qwen3-30B-A3B)
- [Gemma 3 12B IT](https://huggingface.co/google/gemma-3-12b-it)
- [llama.cpp](https://github.com/ggml-org/llama.cpp) ·
  [Ollama](https://docs.ollama.com)
