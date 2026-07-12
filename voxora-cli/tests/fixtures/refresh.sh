#!/usr/bin/env bash
# Refresh the real HF recordings used by the wiremock tests.
#
# Run from anywhere; it operates relative to its own directory.
# Requires Python 3 (stdlib only). No `pip install` needed.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$HERE"

python3 << PY
import json, urllib.request, os, sys

BASE = "https://huggingface.co"
OUT = "$OUT"

models = {
    "qwen3-asr-0.6b": {
        "id": "Qwen/Qwen3-ASR-0.6B",
        "small": ["config.json", "preprocessor_config.json", "tokenizer_config.json"],
        "trio": ["vocab.json", "merges.txt"],
    },
    "qwen3-asr-1.7b": {
        "id": "Qwen/Qwen3-ASR-1.7B",
        "small": ["config.json", "preprocessor_config.json", "tokenizer_config.json",
                  "model.safetensors.index.json"],
        "trio": ["vocab.json", "merges.txt"],
    },
    "whisper-tiny": {
        "id": "openai/whisper-tiny",
        "small": ["config.json", "preprocessor_config.json", "tokenizer_config.json", "tokenizer.json"],
    },
}


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "voxora-fixture-capture/0.1"})
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()


for slug, spec in models.items():
    mid = spec["id"]
    rev = "main"
    d = os.path.join(OUT, slug)
    os.makedirs(d, exist_ok=True)

    meta_url = f"{BASE}/api/models/{mid}/revision/{rev}"
    print(f"[{slug}] GET {meta_url}")
    status, body = fetch(meta_url)
    if status != 200:
        sys.exit(f"  ! HTTP {status}: {body[:200]!r}")
    with open(os.path.join(d, "_metadata.json"), "wb") as f:
        f.write(body)

    for fname in spec.get("small", []):
        url = f"{BASE}/{mid}/resolve/{rev}/{fname}"
        print(f"[{slug}] GET {url}")
        status, body = fetch(url)
        if status != 200:
            sys.exit(f"  ! HTTP {status}: {body[:200]!r}")
        with open(os.path.join(d, fname.replace("/", "__")), "wb") as f:
            f.write(body)

    for fname in spec.get("trio", []):
        url = f"{BASE}/{mid}/resolve/{rev}/{fname}"
        print(f"[{slug}] GET {url}")
        status, body = fetch(url)
        if status != 200:
            sys.exit(f"  ! HTTP {status}: {body[:200]!r}")
        with open(os.path.join(d, fname.replace("/", "__")), "wb") as f:
            f.write(body)

print("done")
PY
