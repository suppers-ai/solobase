# Unified Message + Context System Design

Redesign of `suppers-ai/messages` block from a simple thread/message CRUD into a protocol-agnostic context + entry system that supports chat conversations, A2A task lifecycle, notifications, and future protocols without schema changes.

## Background

Solobase has OpenAPI + A2A AgentCard discovery endpoints (see `2026-04-11-block-schema-discovery-design.md`). The next step is A2A task lifecycle — receiving tasks from external agents, tracking status, returning results.

Rather than creating a separate `suppers-ai/tasks` block, we redesign the existing `suppers-ai/messages` block into a unified model where:

- **Context** replaces "thread" — an abstract container that can be a conversation, an A2A task, a notification channel, or anything else
- **Entry** is the universal primitive — can be a chat message, a task communication, an artifact, a notification, a status change
- There is no artificial distinction between "chat" and "tasks"

## Core Abstractions

### Context

A container that groups related entries. Its `type` field determines what it represents:

- `"conversation"` — a chat thread (LLM block, user chat)
- `"task"` — an A2A task or any unit of work
- `"notification"` — a notification channel
- Any other string a future protocol needs

Status is a free-form string. The block stores it; protocol handlers define valid values and transitions. A conversation might stay `"active"` forever. An A2A task moves through `"submitted"` → `"working"` → `"completed"`.

Contexts can be hierarchical via `parent_id`. Root contexts have `NULL`, sub-tasks or branched conversations point to their parent.

### Entry

The universal primitive within a context. Its `kind` field determines interpretation:

| kind | role | content_type | Use case |
|------|------|-------------|----------|
| `message` | `user` | `text/plain` | Chat message from a user |
| `message` | `agent` | `text/plain` | Agent reply |
| `artifact` | `agent` | `application/pdf` | A2A artifact / task output |
| `notification` | `system` | `text/plain` | System notification |
| `status` | `system` | — | Status change log |

Entries are **append-only**. No update operation — if content needs correction, add a new entry.

The `content` field holds primary text. Protocol-specific structure (A2A parts array, file references, binary data pointers) goes in `metadata`.

## Database Schema

### `suppers_ai__messages__contexts`

| Column | Type | Default | Notes |
|--------|------|---------|-------|
| id | string | auto | Primary key |
| type | string | — | Free-form: `"conversation"`, `"task"`, `"notification"` |
| status | string | `"active"` | Free-form lifecycle state |
| title | string | `""` | Human-readable label |
| sender_id | string | `""` | Who initiated this context |
| recipient_id | string | `""` | Who is responsible / receiving |
| parent_id | string | null | Nullable FK → contexts.id |
| metadata | text | `"{}"` | Protocol-specific data (JSON) |
| created_at | datetime | auto | RFC 3339 |
| updated_at | datetime | auto | RFC 3339 |

**Indexes:** `type`, `status`, `parent_id`, `updated_at`, `sender_id`

### `suppers_ai__messages__entries`

| Column | Type | Default | Notes |
|--------|------|---------|-------|
| id | string | auto | Primary key |
| context_id | string | — | FK → contexts.id |
| kind | string | `"message"` | `"message"`, `"artifact"`, `"notification"`, `"status"` |
| role | string | `""` | `"user"`, `"agent"`, `"system"`, or empty |
| status | string | `""` | Free-form delivery status |
| sender_id | string | `""` | Who created this entry |
| content | text | `""` | Primary text content |
| content_type | string | `"text/plain"` | MIME type |
| metadata | text | `"{}"` | Protocol-specific data (parts, file refs, etc.) |
| created_at | datetime | auto | RFC 3339 |

**Indexes:** `context_id`, `(context_id, created_at)`, `kind`, `(context_id, kind)`

No `updated_at` on entries — they are append-only.

## Architecture: Shared Service Layer

Both REST and A2A handlers call shared service functions. No HTTP-level indirection, no business logic duplication.

```
                 ┌─────────────┐
                 │   mod.rs    │
                 │  (routing)  │
                 └──────┬──────┘
              ┌─────────┼─────────┐
              v         v         v
        ┌──────────┐ ┌───────┐ ┌──────────┐
        │  rest.rs │ │a2a.rs │ │ pages.rs │
        │  (REST)  │ │(JSONR)│ │  (SSR)   │
        └────┬─────┘ └───┬───┘ └────┬─────┘
             └────────┬───┘         │
                      v             v
               ┌─────────────┐  (reads via
               │ service.rs  │   service)
               │  (business  │
               │   logic)    │
               └──────┬──────┘
                      v
               ┌─────────────┐
               │  database   │
               └─────────────┘
```

### Service Functions

**Context operations:**

- `create_context(ctx, type, title, sender_id, recipient_id, parent_id?, metadata?)` → Context
- `get_context(ctx, id)` → Context
- `list_contexts(ctx, filters?, sort?, pagination?)` → Page<Context>
- `update_context(ctx, id, updates)` → Context — updates status, title, metadata; stamps `updated_at`
- `delete_context(ctx, id)` → () — cascade-deletes all entries, then deletes context

**Entry operations:**

- `add_entry(ctx, context_id, kind, role, sender_id, content, content_type?, metadata?)` → Entry — bumps parent context's `updated_at`
- `get_entry(ctx, id)` → Entry
- `list_entries(ctx, context_id, filters?, pagination?)` → Page<Entry> — sorted by `created_at` ASC
- `delete_entry(ctx, id)` → ()

These are plain async functions. No HTTP awareness — they take typed arguments and return `Result<Value, Error>`.

## REST API Endpoints

All require `AuthLevel::Authenticated`. All endpoints declare JSON Schemas on their `BlockEndpoint` definitions.

### Context endpoints

| Method | Path | Service call |
|--------|------|-------------|
| GET | `/b/messages/api/contexts` | `list_contexts` |
| POST | `/b/messages/api/contexts` | `create_context` |
| GET | `/b/messages/api/contexts/{id}` | `get_context` |
| PATCH | `/b/messages/api/contexts/{id}` | `update_context` |
| DELETE | `/b/messages/api/contexts/{id}` | `delete_context` |

**GET `/b/messages/api/contexts`** query params: `?type=task&status=working&sender_id=xxx&parent_id=xxx&page=1&page_size=20`

### Entry endpoints

| Method | Path | Service call |
|--------|------|-------------|
| GET | `/b/messages/api/contexts/{id}/entries` | `list_entries` |
| POST | `/b/messages/api/contexts/{id}/entries` | `add_entry` |
| GET | `/b/messages/api/entries/{id}` | `get_entry` |
| DELETE | `/b/messages/api/entries/{id}` | `delete_entry` |

**GET `/b/messages/api/contexts/{id}/entries`** query params: `?kind=artifact&role=agent&page=1&page_size=100`

9 endpoints total (down from 10 — no PATCH on entries since they are append-only).

## A2A JSON-RPC Endpoint

`POST /a2a` — single endpoint, dispatches by JSON-RPC `method` field. `AuthLevel::Authenticated`.

### Method mapping

| A2A Method | Service calls | Notes |
|------------|--------------|-------|
| `SendMessage` | `create_context(type="task", status="submitted")` + `add_entry(kind="message", role="user")` | Creates context + first entry if new. If `contextId` provided, adds entry to existing context |
| `GetTask` | `get_context(id)` + `list_entries(context_id)` | Maps context → A2A Task, entries → Messages/Artifacts by `kind` |
| `ListTasks` | `list_contexts(type="task", ...)` | Filters by status, contextId as parent_id. Cursor-based pagination |
| `CancelTask` | `update_context(id, status="canceled")` | Fails if context is already in a terminal status |

### A2A concept mapping

| A2A concept | Internal representation |
|-------------|----------------------|
| Task | Context with `type="task"` |
| Task.status | Context `status` (values: `submitted`, `working`, `completed`, `failed`, `canceled`, `input_required`) |
| Task.contextId | Context `parent_id` (groups related tasks) |
| Message | Entry with `kind="message"` |
| Message.role | Entry `role` (`"user"` or `"agent"`) |
| Message.parts | Entry `content` (primary text) + `metadata.parts` (full parts array) |
| Artifact | Entry with `kind="artifact"`, `content_type` set to artifact's mimeType |

The A2A handler transforms internal Context/Entry objects into A2A Task/Message/Artifact JSON shapes before returning. Callers see standard A2A responses.

### Not in scope (first pass)

- Streaming/SSE (`SubscribeToTask`)
- Push notifications
- AgentCard already declares `"streaming": false, "pushNotifications": false`

## LLM Block Migration

The LLM block currently calls `messages_create()` and `messages_list()` helpers that hit thread/message endpoints. Update these to call the new endpoints:

- `messages_create(thread_id, role, content)` → `POST /b/messages/api/contexts/{id}/entries` with `kind="message"`
- `messages_list(thread_id)` → `GET /b/messages/api/contexts/{id}/entries?kind=message`
- Thread creation → `POST /b/messages/api/contexts` with `type="conversation"`

Response field names change, so parsing needs updating. Logic stays the same — mechanical find-and-replace, not a redesign.

Additionally, `solobase-core/src/blocks/llm/pages.rs` has duplicated collection name constants (`THREADS_COLLECTION`, `MESSAGES_COLLECTION`) that reference the old tables directly for SSR page rendering. These must be updated to the new collection names (`suppers_ai__messages__contexts`, `suppers_ai__messages__entries`).

## UI Pages Migration

Admin SSR pages update to use contexts/entries terminology and endpoints.

- **Context list page** (`GET /b/messages/`) — shows type and status badges alongside title. HTMX form creates a context.
- **Context detail page** (`GET /b/messages/contexts/{id}`) — renders entries styled by `kind`. Entry form includes kind selector (defaults to message).

Same admin-only access pattern.

## File Structure

```
messages/
├── mod.rs          # Block struct, BlockInfo, collection schemas, endpoint declarations, routing
├── service.rs      # Core service functions (create_context, add_entry, etc.)
├── rest.rs         # REST endpoint handlers
├── a2a.rs          # A2A JSON-RPC handler
└── pages.rs        # SSR admin pages (maud templates)
```

## Database Migration

Clean replacement — no backward compatibility needed (active development, no production data).

1. Replace `CollectionSchema` declarations in `BlockInfo::info()` with new table definitions
2. WAFER runtime creates new tables from schema declarations
3. No data migration script — existing dev data is abandoned

There are no separate SQL migration files. Tables are created declaratively via `CollectionSchema` in `BlockInfo::info()`. Replacing the schema declarations is sufficient — new setups will only create the new tables. Old tables won't be created because no code declares them.

**Files that reference old collection names** (all must be updated):
- `solobase-core/src/blocks/messages/mod.rs` — defines `THREADS_COLLECTION` and `MESSAGES_COLLECTION` constants, uses them in CRUD handlers and `CollectionSchema` declarations
- `solobase-core/src/blocks/messages/pages.rs` — imports and uses the constants for SSR pages
- `solobase-core/src/blocks/llm/pages.rs` — has duplicated constants for direct table reads in SSR pages
- `solobase-core/src/blocks/llm/mod.rs` — calls messages block via HTTP endpoints (URL paths change, not table names)
