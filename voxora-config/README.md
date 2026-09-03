# voxora-config

Single source of truth for every [voxora](https://github.com/airvzxf/voxora)
runtime setting: the model cache root plus the Hugging Face token, base
URL, and default revision. ASR-specific by design — it models what the
voxora speech-recognition stack needs, not LLM or vision settings.

Every setting resolves through one cascade, first non-empty wins: an
explicit value on `VoxoraConfig` (set by the caller, or read from a TOML
file with `VoxoraConfig::from_file`), then the environment, then the
built-in default.

Variables honoured: `VOXORA_CACHE_DIR`, `VOXORA_HF_TOKEN`,
`VOXORA_HF_BASE_URL`, `VOXORA_HF_REVISION`, plus the `HF_TOKEN` and
`HUGGING_FACE_HUB_TOKEN` aliases. Per-setting rules: [module docs](https://docs.rs/voxora-config).
