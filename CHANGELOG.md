# Changelog

All notable changes to CoCompute will be documented in this file.

## [0.5.5] - 2026-04-03

### Changed
- Split `host/src/main.rs` into focused modules: `connection`, `handlers`, `ollama`, `update`
- Split `orchestrator/src/main.rs` into `proxy` and `routes/{chat, embeddings, models, stats, system}`
- Split `orchestrator/src/db/migration.rs` into individual migration files under `migrations/`
- Extract shared Ollama conversion helpers (`convert_messages`, `convert_tools`, `convert_tool_calls`) into reusable functions

### Removed
- Tool artifacts (`.pi/`, `context.md`) from version control
