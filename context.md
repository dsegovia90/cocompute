Written to `/Users/daniel/projects/closedsource/cocompute/context.md`. Here's a summary of what was found:

---

## Findings

### 🔴 Two files need splitting

**`host/src/main.rs` (822 lines)** — monolithic file mixing 5 distinct concerns:
- CLI + startup + reconnect loop
- Ollama inference handlers (embeddings, chat, streaming chat)
- Self-update logic (check, download, atomic swap)
- Heartbeat task (embedded inside `connect_and_serve`)
- Ollama conversion helpers — **duplicated verbatim** between `handle_chat` and `handle_chat_stream` (~60 lines of copy-paste for message/tool building)

**`orchestrator/src/main.rs` (625 lines)** — mixes:
- App setup + router wiring
- Proxy helpers (`send_to_host`, `route_to_host`, `log_metering`)
- All 7 HTTP route handlers, including `create_chat_completion_stream` which alone is ~130 lines

### 🟡 Minor issues
- `orchestrator/src/openai.rs` (377 lines): well-structured but the test suite is 200+ lines; fine as-is
- `orchestrator/src/db/migration.rs` (182 lines): all 4 migrations in one file — low priority since migrations rarely change

### 🟢 Everything else is well-sized
`host_manager.rs`, `host_acceptor.rs`, `auth.rs`, `error.rs`, `common/` — all reasonable

### Proposed splits
- **host**: `main.rs` → `connection.rs`, `handlers.rs`, `ollama.rs`, `chat.rs`, `embeddings.rs`, `update.rs`
- **orchestrator**: `main.rs` → `proxy.rs`, `routes/models.rs`, `routes/embeddings.rs`, `routes/chat.rs`, `routes/system.rs`