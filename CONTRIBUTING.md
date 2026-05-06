# Contributing to cocompute

Thanks for considering a contribution. Read this first to avoid surprises.

## Before you open a PR

cocompute requires a Contributor License Agreement (CLA). This is non-negotiable and exists for one reason: so the project can offer a non-AGPL license to the hosted operator (cocompute.io) without forcing all derivatives to inherit AGPLv3 §13 disclosure.

The CLA is enforced by [cla-assistant](https://cla-assistant.io/dsegovia90/cocompute). When you open a PR, a bot comments with the CLA text and a link to sign. You sign once per GitHub account, and the bot remembers you across all your PRs. The CLA does not change the AGPL license under which you receive the code; it grants the project the right to relicense your contribution.

If you cannot sign the CLA, please open an issue describing what you wanted to contribute. We can usually find a way (most often: someone else implements the same change from scratch).

## Issues

For bugs, please include:

- The version of `cocompute_orchestrator` (`cocompute_orchestrator --version`) or `cocompute_host` (`cocompute_host --version`)
- Your OS and architecture
- A minimal repro: the exact command, the exact request, the exact response, the relevant logs
- What you expected vs what happened

For feature requests, please include:

- The user-visible problem you're trying to solve (not the proposed implementation)
- Why the current behavior doesn't work for your use case
- Whether you'd be willing to send a PR

## Development setup

You need Rust stable (current MSRV is 2024 edition, so any recent toolchain) and Ollama installed locally.

```sh
git clone https://github.com/dsegovia90/cocompute
cd cocompute
cargo build
```

The repo uses [bacon](https://dystroy.org/bacon/) for live recompilation and [just](https://just.systems/) for shortcut commands. Both are optional but recommended.

```sh
cargo install bacon just
bacon          # live cargo check
just           # see available commands
```

To run a local orchestrator and one host on the same machine:

```sh
# Terminal 1 — orchestrator (creates cocompute.db on first run)
cargo run -p cocompute_orchestrator

# Terminal 2 — generate an admin API key for testing
cargo run -p cocompute_orchestrator -- generate-key

# Terminal 3 — host (replace YOUR_TOKEN with one from the dashboard "Add Host" flow)
cargo run -p cocompute_host -- --orchestrator-url http://localhost:4000 --setup-token YOUR_TOKEN
```

The orchestrator listens on `http://localhost:4000` by default. Visit it in a browser to see the landing page. The dashboard is at `/dashboard` after you sign up at `/beta`.

Email features (verification, invites) require SMTP. For local dev, run [Mailpit](https://github.com/axllent/mailpit) on the default port (1025) and the orchestrator will pick it up automatically. Without Mailpit, the orchestrator logs the verification URLs to stdout instead of sending email.

## Code conventions

Match the surrounding code. A few rules that aren't obvious:

- Use `tracing` (not `println!`) for runtime output. CLI subcommand output (e.g., `generate-key`) is the one exception.
- Soft-deletable tables (`pools`, `api_keys`, `host_pool_memberships`) require an `is_active = true` filter on every read query unless you are intentionally reading archived rows. See ARCHITECTURE.md for the rationale.
- Don't add `unwrap()` or `expect()` to request handlers. Return a `Response` with appropriate redirect-with-error or HTTP status.
- Tests live alongside the code (`#[cfg(test)] mod tests`) for unit tests and in `crate/tests/` for integration tests.
- Run `cargo fmt` before opening a PR.
- Run `cargo clippy --workspace --all-targets` and fix any new lints you introduce.

## Commit style

Short, lowercase, scope-light, present tense. Examples from the repo's history:

```
allow naming of keys and hosts
fix: use ephemeral iroh keys to prevent stale DNS disconnections
turnstile
github icon
```

Group logically. One commit per concern. Squash before merge if reviewers ask.

## PR process

1. Open the PR against `master`. Smaller PRs land faster.
2. The CLA bot will comment with a signing link if it doesn't recognize you yet.
3. CI runs `cargo check`, `cargo test`, `cargo clippy`, and `cargo fmt --check`. All must pass.
4. A maintainer reviews. Iteration is normal. We tend to be picky about handler error paths and database query correctness.
5. Once approved and CI is green, a maintainer merges. We use squash merge by default.

## What we will not accept

- Cryptocurrency or token incentive systems. cocompute uses pool-credit accounting; tokens are not on the roadmap and the architecture is intentionally hostile to them.
- Telemetry or analytics that send data off the operator's machine without explicit opt-in.
- Closed-source binary blobs as dependencies.
- Mass-email or notification features that aren't gated behind explicit user opt-in.
- Code that bypasses the AGPL license boundary (e.g., dlopen-based plugin systems designed to escape copyleft).

## Security

Please do not file public issues for security vulnerabilities. Email security@cocompute.io or open a private security advisory on GitHub. We will respond within 72 hours and coordinate disclosure.

## License

By contributing, you agree that your contributions will be licensed under AGPLv3 (the project's license) and that you grant the project the right to dual-license your contribution under non-AGPL terms (per the CLA). You retain copyright on your contribution.
