# Changelog

All notable changes to cocompute will be documented in this file.

## [0.7.2] - 2026-06-02

### Changed
- Reframed the landing page around the core job: safely exposing your local model as an OpenAI-API-compatible endpoint. New code-first hero shows the one-line host command alongside a connection card (base URL + key) listing compatible tools (Open WebUI, AnythingLLM, Jan, LibreChat, LangChain, Continue, n8n).
- Renamed the public signup route from `/beta` to `/signup`. The old `/beta` link still redirects, so existing links keep working. Signup page, verification emails, and login copy no longer say "beta" now that signup is open.
- Replaced em-dashes across the codebase with commas, and page titles with a middle-dot separator (`cocompute · X`), per project style.

### Added
- Reusable `CodeBlock` (with copy-to-clipboard) and `CodeWindow` UI components for rendering commands and code.
- A "Why we built cocompute" section on the landing page explaining the mission: make open models easy to run, and build toward an open compute marketplace.

### Removed
- Dead `waitlist_email` template and stale waitlist copy.

## [0.5.5] - 2026-04-03

### Changed
- Split `host/src/main.rs` into focused modules: `connection`, `handlers`, `ollama`, `update`
- Split `orchestrator/src/main.rs` into `proxy` and `routes/{chat, embeddings, models, stats, system}`
- Split `orchestrator/src/db/migration.rs` into individual migration files under `migrations/`
- Extract shared Ollama conversion helpers (`convert_messages`, `convert_tools`, `convert_tool_calls`) into reusable functions

### Removed
- Tool artifacts (`.pi/`, `context.md`) from version control
