# cocompute

[![License: AGPLv3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](./LICENSE)
[![Status](https://img.shields.io/uptimerobot/status/m803005443-b162fae4bda6b2fff299e3a0?label=cocompute.ai)](https://stats.uptimerobot.com/hdrVVZOlHE)
[![Uptime (7d)](https://img.shields.io/uptimerobot/ratio/7/m803005443-b162fae4bda6b2fff299e3a0)](https://stats.uptimerobot.com/hdrVVZOlHE)

Open infrastructure for cooperative LLM inference on consumer hardware.

cocompute lets you share an idle GPU sitting on your home network, and use other people's GPUs through an OpenAI-compatible API. Anything that runs Ollama works (NVIDIA, AMD, Apple Silicon, CPU). NAT traversal is handled by [iroh](https://www.iroh.computer/), so no port forwarding, no router config, no public IP required.

The protocol is open and AGPL-licensed. The same code that runs at [cocompute.ai](https://cocompute.ai) is the code in this repo. cocompute.ai is the hosted version for people who don't want to operate their own orchestrator.

```
┌──────────────┐                ┌────────────────────┐                ┌──────────────┐
│ host         │  iroh / QUIC   │ orchestrator       │  HTTP / JSON   │ consumer     │
│ (your GPU)   │ ◄──────────────►│ (cocompute.ai      │ ◄──────────────►│ (curl, SDK,  │
│ runs Ollama  │                │  or self-hosted)   │                │  openwebui)  │
└──────────────┘                └────────────────────┘                └──────────────┘
```

## Status

Pre-launch alpha. Expect breaking changes between minor versions until 1.0. The protocol is stabilizing but not frozen.

The badges at the top of this README show real-time status and uptime for cocompute.ai. They're hosted off-domain (shields.io + UptimeRobot) so they stay reachable when cocompute.ai itself is down. Full history at the [public status page](https://stats.uptimerobot.com/hdrVVZOlHE).

## Quick start

You can be a host (share your GPU), a consumer (use the pool), or both.

**Share your GPU:** sign up, click "Add Host" in the dashboard for a one-line install command, run it on any machine that runs Ollama. Service installs via systemd (Linux) or launchd (macOS) and stays running in the background.

```sh
curl -sSf https://cocompute.ai/install.sh | COCOMPUTE_URL=https://cocompute.ai bash -s -- --token YOUR_TOKEN
```

(Future) Reciprocity is tracked by pool-credit accounting. No tokens, no crypto.

**Use the pool (no public pools yet):** sign up at [cocompute.ai/quickstart](https://cocompute.ai/quickstart), create an API key, point any OpenAI-compatible client at `https://cocompute.ai/v1/`. Two minutes if you have curl handy.

```sh
curl https://cocompute.ai/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"llama3.2","messages":[{"role":"user","content":"hello"}]}'
```

## Self-hosting

You can run your own orchestrator and host fleet. The orchestrator is a single Rust binary backed by SQLite. See [ARCHITECTURE.md](./ARCHITECTURE.md) for the moving parts and [docs/self-hosting.md](./docs/self-hosting.md) for setup steps. Build from source for now (`cargo build --release -p cocompute_orchestrator`); container images are on the roadmap.

## Why open source?

Inference infrastructure is becoming the substrate for everything. A handful of vendors deciding what models you can run, and at what price, is not the future we want. The protocol that lets a 3090 in someone's apartment serve LLM inference to a laptop in a coffee shop should be open, forkable, and operable by anyone. cocompute.ai is the convenient default; the code is the guarantee.

cocompute is licensed AGPLv3. Contributors sign a CLA so the project can offer a commercial license to the hosted operator (cocompute.ai) without forcing all derivatives to inherit AGPL semantics. See [CONTRIBUTING.md](./CONTRIBUTING.md) for the full story.

## Contributing

Issues, PRs, and discussions welcome. Please read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening a PR. All contributions require signing the CLA via cla-assistant.

## Acknowledgments

cocompute stands on the shoulders of:

- [iroh](https://www.iroh.computer/) for peer-to-peer transport and NAT traversal
- [Ollama](https://ollama.com/) for the local model runtime
- [Axum](https://github.com/tokio-rs/axum), [Leptos](https://leptos.dev/), [Sea-ORM](https://www.sea-ql.org/SeaORM/), [Tokio](https://tokio.rs/) for the orchestrator stack

## License

[AGPLv3](./LICENSE). Source code is available; modifications running as a network service must also be available to users of that service. Contributors agree via CLA that the project can dual-license to the hosted operator.
