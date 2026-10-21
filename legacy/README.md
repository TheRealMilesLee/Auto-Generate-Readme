# legacy implementations

These files are kept for reference and are **not maintained**:

- `python-template/` — the original Python/Jinja2 attempt. It does not parse
  source code, has syntax errors in `core.py` / `config.py`, and is missing its
  console entry point.
- `run.sh` — the original Bash + Ollama generator. It sends a directory tree to
  the model, then removes "thinking" with 67 `grep -v` filters.

The Rust implementation in `src/` replaces both. If you need the old behaviour,
check out the commit before the Rust rewrite; this directory will be deleted in
a future cleanup.
