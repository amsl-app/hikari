## Commands

```bash
# Build
cargo build --workspace

# Check/lint/format/test (run after any code change)
cargo clippy --workspace --fix --allow-dirty && cargo fmt --all && cargo test --all

# Run a single test
cargo test -p <crate-name> <test_name>

# Run the development server (requires .env file and podman)
sh tools/run.sh
```

## Architecture

Hikari is a chatbot backend built in Rust. It runs as two services:

- **`hikari-server`** (port 3030): Main HTTP API built with Axum. Handles OIDC authentication, CSML bot execution, document/collection management, and proxies LLM requests.
- **`hikari-worker`** (port 3035): Separate Axum service that handles async CSML execution and journaling. Exposes Prometheus metrics.

Both services connect to a PostgreSQL database (with `pgvector` extension) and optionally S3-compatible storage for config/document loading.

### Crate Map

| Crate | Role |
|---|---|
| `hikari-server` | Main API entry point, auth, routes, CSML bot loading |
| `hikari-worker` | Worker service for async bot execution and journaling |
| `hikari-llm` | LLM pipeline: YAML-driven agent builder + execution engine |
| `hikari-core` | Low-level primitives: OpenAI calls, pgvector RAG, journaling, quiz |
| `hikari-config` | Config types for modules, assessments, documents, constants, global config |
| `hikari-db` | SeaORM database access layer |
| `hikari-entity` | SeaORM entity definitions |
| `hikari-model` | Shared domain types (messages, LLM models, etc.) |
| `hikari-model-tools` | Axum extractors, SSE, tool/slot helpers for routes |
| `hikari-http` | Shared HTTP client with retry logic |
| `hikari` | OIDC client and high-level bot API |
| `hikari-oidc` | JWT/OIDC token validation |
| `hikari-common` | Shared CSML utilities and error types |
| `hikari-utils` | Tracing setup, S3/file loader, CLI arg helpers, tower middleware |
| `hikari-tools` | CLI tool for inspecting/testing LLM structures |
| `hikari-cli` | REPL for testing CSML bots locally |
| `hikari-test-helpers` | Shared test utilities |

### LLM Pipeline

The LLM agent system in `hikari-llm` is YAML-driven:

1. **Builder** (`hikari-llm/src/builder/`): Deserializes YAML config into typed step definitions. Key types: `LlmBuilder` (an LLM step), `LlmStructureConfig` (top-level agent config). Steps support prompts, slots (input extraction), memory filters, tool use, conditions, and document injection. `{{variable}}` placeholders in prompts are resolved from slots at runtime via `InjectionTrait`.

2. **Execution** (`hikari-llm/src/execution/`): `LlmCore` assembles chat messages from conversation history + slots + prompts, then calls OpenAI-compatible APIs via `hikari-core`. `LlmAgent` drives the step loop — it iterates `LlmStepIterator`, executes each `LlmStep`, and streams results back via `handle_response`.

**Step content variants** (returned from each step's `execute`):
- `LlmStepContent::Message { message, store }` — streams an LLM response token-by-token via `MessageStream`; the agent buffers into `complete_message` and emits `Response::Chat(ChatChunk)` chunks while writing to the DB.
- `LlmStepContent::StepValue { values, next_step }` — sets one or more slots, optionally jumps to another step.
- `LlmStepContent::Combined(steps)` — runs multiple sub-contents in sequence (used by `CombinedStep`).
- `LlmStepContent::Skipped` — condition not met; step is skipped.

**WebSocket chat API**: the LLM conversation is driven over WebSocket at `/chat/{module_id}/{session_id}/ws`. The server sends `Response` variants (`Typing`, `Chat(ChatChunk)`, `Hold`, `ConversationEnd`, `History`, `Error`) and accepts `Request::Chat`, `Request::ConnectionInfo`, and `Request::Abort` messages.

### Config Loading

Configs (modules, CSML bots, assessments, constants, LLM structures, document collections) are loaded from either local filesystem paths or S3 URLs via `hikari-utils::loader::LoaderHandler`. The `--config`, `--csml`, `--llm-structures`, etc. CLI args accept `file://` or `s3://` URLs.

### Database

Uses SeaORM with PostgreSQL. Diesel is used only for migrations in `hikari-server`. The `pgvector` extension is required for RAG document embedding and similarity search.

## Logging

Use the `tracing` crate. Do not interpolate variables into the format string — pass them as fields:
```rust
tracing::error!(%err, "Error description");
tracing::debug!(%user_id, "Processing request");
```

- `tracing::debug`: Detailed diagnostic information and execution tracing.
- `tracing::warn`: Unexpected situations that don't halt execution but need attention.
- `tracing::error`: Critical failures and error states.

## Workspace Lints

`clippy::unwrap_used` is **denied** workspace-wide. Use `?` or `.expect()` with a message. `clippy::indexing_slicing` is also denied — use `.get()` with proper error handling.

Additional warned lints: `clone_on_ref_ptr`, `implicit_clone`, `rc_buffer`, `rc_mutex`, `string_add`, `todo`, `dbg_macro`, `unreachable`, `unimplemented`. **Prefer borrowing over cloning** — avoid unnecessary `.clone()` calls.
