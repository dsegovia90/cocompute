# Architecture

cocompute is a three-piece Rust workspace.

```
cocompute/
├── common/         protocol types, helpers, shared serialization
├── host/           the host binary (runs on a machine where inference is done)
└── orchestrator/   the orchestrator (web UI, API, host registry, routing)
```

## Data flow

```
HOST (consumer hardware, behind NAT)            ORCHESTRATOR (cocompute.io or self-hosted)
                                                     |
  cocompute_host binary                              | Axum HTTP (default port 4000)
  ├─ iroh::Endpoint (ephemeral key)                  | Leptos SSR pages (landing, dashboard, ...)
  ├─ Ollama proxy (localhost:11434)                  | Sea-ORM + SQLite (cocompute.db)
  └─ orchestrator client                             | HostManager (in-memory connected hosts)
       |                                             | iroh::Endpoint (server side)
       | (1) GET /v1/node-info                       |
       |---------- HTTP --------------------------- >| (returns orchestrator's iroh node id)
       | (2) iroh::connect(orchestrator_id)          |
       |========== QUIC over iroh ================= >| host_acceptor.rs
       | (3) RegistryRequest::Register(token)        |   restores pool memberships from DB
       | (4) bidirectional inference stream          |   inserts/updates host record
       |     - ChatRequest -> Ollama -> response     |
       |     - capabilities heartbeat                |
       |                                             |
                                                     |
  CONSUMER (developer with API key)                  |
  └─ POST /v1/chat/completions                       |
       | Authorization: Bearer cck_...               |
       |---------- HTTP --------------------------- >| auth::require_api_key middleware
                                                     |   loads ApiKey, sets PoolContext extension
                                                     | proxy.rs
                                                     |   selects host from pool (round-robin)
                                                     |   forwards over iroh stream
                                                     |   logs to metering_logs
```

## Crate responsibilities

### `common/`

Wire protocol types and shared helpers. Both host and orchestrator depend on it.

- `protocols/registry.rs` — `RegistryRequest`/`RegistryResponse`: how a host registers with the orchestrator and what capabilities it announces.
- `protocols/chat.rs` and `protocols/embeddings.rs` — internal request/response types for the bidirectional inference streams over iroh.
- `helpers/` — `read_p2p` and `write_p2p` framing for bitcode-encoded messages over iroh streams.

### `host/`

The binary that runs on user hardware. Single CLI (`cocompute_host`).

- `main.rs` — argument parsing, persistent host_id (lives at `~/.cocompute/host_id`), reconnect loop with exponential backoff.
- `connection.rs` — establishes the iroh QUIC connection to the orchestrator and serves the bidirectional inference stream.
- `ollama.rs` — wraps the local Ollama HTTP API. Translates between the cocompute internal protocol and Ollama's JSON.
- `handlers.rs` — per-request handler logic for chat and embeddings.
- `update.rs` — self-update flow. Fetches a newer host binary from `$ORCHESTRATOR/v1/update/$PLATFORM` and replaces the current binary atomically.

### `orchestrator/`

Larger crate. Web UI plus API plus host acceptor plus DB.

- `main.rs` — CLI args (port, db, SMTP, Turnstile, etc.), Axum router setup, iroh acceptor wiring, AppState construction.
- `host_acceptor.rs` — the iroh `ProtocolHandler`. Accepts incoming host connections, validates setup tokens, restores pool memberships, registers the host in the in-memory `HostManager`.
- `host_manager.rs` — in-memory map of currently-connected hosts. Tracks capabilities (which models each host serves), pool membership, and provides round-robin host selection for inference requests.
- `auth.rs` — API key generation and hashing (SHA-256), the `require_api_key` middleware that loads `ApiKeyId` and `PoolContext` into request extensions, password hashing (argon2), session cookie helpers.
- `proxy.rs` — forwards an incoming HTTP inference request to a connected host over the iroh stream, streams the response back.
- `openai.rs` — translation between OpenAI's wire format and cocompute's internal types.
- `routes/` — HTTP route handlers (`/v1/chat/completions`, `/v1/embeddings`, `/v1/models`, `/v1/stats`, `/v1/node-info`, `/v1/version`, `/v1/update/{platform}`).
- `web/` — Leptos SSR pages and POST handlers for the dashboard, signup, host management.
- `db/` — Sea-ORM entities and migrations. SQLite by default; the schema runs on Postgres too if you swap the connection string.
- `email/` — SMTP-based mailer for verification and invite emails. Optional; if `SMTP_HOST` is unset, email features are disabled.

## Key concepts

**Host.** A machine running `cocompute_host`. Identified by a stable UUID (`host_id`) persisted at `~/.cocompute/host_id`. The iroh `endpoint_id` is ephemeral and changes on restart; the orchestrator keys hosts by `host_id` so reconnection is seamless.

**Pool.** A named group of hosts. Pool owners can invite members and add hosts. API keys are scoped to a pool; `/v1/models` and `/v1/chat/completions` only see hosts in the API key's pool. There is also a global pool that anyone can join.

**API key.** SHA-256 hash stored in `api_keys` table. Scoped to a pool. Required for any `/v1/*` route. Auth happens in `auth::require_api_key` middleware.

**Soft-deletion.** `pools`, `api_keys`, and `host_pool_memberships` have an `is_active boolean` column. Removing a thing flips this to false; the row stays for history. Every query against these tables MUST filter `is_active = true` unless intentionally fetching archived rows (e.g., reactivating). See the inline comments in `host_acceptor.rs::restore_pools` for the why.

**Setup token.** One-time token issued by the orchestrator that establishes a host's user ownership on first connection. After token use, the host_id alone is sufficient to reconnect.

## Open-core boundary

This repository is the AGPL open core. A separate private repository (`cocompute-cloud`, not in this monorepo) contains the proprietary additions that run only on cocompute.io:

- Stripe billing and metering enforcement
- Verified host program features
- Enterprise SSO
- Abuse monitoring beyond the basic rate limit

The proprietary repo depends on this one as a Cargo path. Contributors sign a CLA so the project can dual-license the hosted version under non-AGPL terms internally without triggering AGPLv3 §13 disclosure for billing code.

If you self-host, you only get the AGPL features. That is intentional and the entire feature surface required to run a cooperative pool.

## Storage

SQLite by default. The connection string in `main.rs` is:

```
sqlite://./cocompute.db?mode=rwc
```

Override via `COCOMPUTE_DB_PATH`. To run on Postgres, swap the URL to `postgres://...` and rebuild with the appropriate Sea-ORM feature set (currently `sqlx-sqlite`; adjust `Cargo.toml`).

## Transport

[iroh](https://www.iroh.computer/) handles peer-to-peer QUIC connections with built-in NAT traversal (hole punching, relay fallback). The orchestrator runs `iroh::Endpoint` in server mode; hosts dial it. The connection is bidirectional: hosts open per-request streams to receive inference work and send responses.

cocompute uses iroh's default `presets::N0` configuration. Identity keys are ephemeral on the host side; the persistent `host_id` is application-level (not iroh's secret key). This avoids stale pkarr DNS entries that previously caused reconnect issues with persisted iroh keys.

## Why this stack

- **Rust** for memory safety in code that handles arbitrary network traffic and untrusted input.
- **Axum** for the HTTP layer because tokio-native, ergonomic extractors, no surprises.
- **Leptos SSR** for the web UI because it's Rust-on-the-server-rendering-real-HTML, no JavaScript build step, no React hydration tax. The dashboard is fast and ships as part of the orchestrator binary via `include_bytes!`.
- **Sea-ORM** because it lets us start on SQLite and migrate to Postgres later without rewriting queries.
- **iroh** because it makes the home-network-to-internet path tractable. Without iroh's hole punching, every host operator would need to configure port forwarding. That alone would kill adoption.
- **Ollama** as the local model runtime because it's already on most self-hosters' machines and exposes a stable HTTP API. cocompute is a thin coordination layer on top of where the actual inference happens.

## Roadmap (not commitments)

- Container images for the orchestrator (GitHub Container Registry)
- Per-API-key rate limiting (current: per-IP via tower-governor)
- Pluggable model backends beyond Ollama (any OpenAI-compatible server, ie: llama.cpp, etc)
- Federation between orchestrators
- Per-host opt-out for content categories
- Open source library for host to consumer communication, no orchestrator needed
