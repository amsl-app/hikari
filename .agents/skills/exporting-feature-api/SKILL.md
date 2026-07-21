---
name: exporting-feature-api
description: Use when asked to export, document, or write up a Hikari feature's HTTP API for a client app/agent — e.g. "export the planner API for the client", "write PLANNER-API.md", "document the journal endpoints for the frontend".
---

# Exporting a Feature's API to `<FEATURE>-API.md`

## Overview

Produces a single self-contained Markdown file (`<FEATURE>-API.md`, e.g. `PLANNER-API.md`) that a
client-side coding agent can implement against **without reading any Rust source**. The OpenAPI
spec alone is not enough — it omits which `Permission` an endpoint needs, whether an error response
actually has a JSON body, and the tri-state PATCH semantics this codebase uses. This skill closes
those gaps.

Path/method accuracy of the generated OpenAPI itself is keeping-routes-openapi-in-sync's job, not
this skill's — this skill assumes that discipline has kept the spec trustworthy.

Output location: repo root, unless the user says otherwise.

## Procedure

1. **Find the feature's routes.** `crates/hikari-server/src/routes/api/v0/<feature>.rs` (+ submodules,
   + a sibling `error.rs`). Note the router prefix it's mounted under and every `#[utoipa::path(...)]`
   handler.

2. **Generate ground-truth OpenAPI.** Don't hand-transcribe structs — utoipa's `ToSchema` derive is
   the source of truth for wire shape (renames, flattening, etc.):
   ```
   cargo run -p hikari-server -- openapi --format pretty > /tmp/openapi.json
   ```
   This needs no DB/.env — the `openapi` subcommand short-circuits before any connection setup.
   Filter to paths tagged `v0/<feature>` and pull every schema they (transitively) reference.

3. **Fill the gaps OpenAPI can't show**, per endpoint, by reading the handler source:
   - **Permission**: `#[protect("Permission::X", ty = "Permission")]` above the handler. No
     `#[protect]` at all means the route is unauthenticated (e.g. a token-in-path feed URL) —
     state this explicitly, don't assume every route needs the bearer token just because the tag
     has a `security` block.
   - **Real error body**: find the feature's `error.rs` and read its `IntoResponse` impl. Most
     Hikari error enums return a bare `StatusCode` with **no JSON body** — `utoipa::path` response
     descriptions are prose only, they don't imply a body exists. Say plainly "no response body"
     where that's true; don't invent an `{"error": "..."}` shape.
   - **Non-JSON responses**: check for a `content_type` override (e.g. `text/calendar`) and document
     the raw format instead of a JSON schema.
   - **Tri-state PATCH fields**: a field typed `Option<Option<T>>` with
     `#[serde(default, with = "::serde_with::rust::double_option")]` means three distinct states —
     *field omitted* (no change), *field: null* (clear it), *field: value* (set it). Flag every such
     field by name; this is the single most common client bug.

4. **Write the Data Models section** from the resolved OpenAPI schemas, one table per struct: field,
   JSON type, nullable?, notes. Apply the wire-format table below rather than the Rust type name.

5. **Write the Endpoints section**, one subsection per handler, in this order: Method + Path →
   Auth/Permission → Path/Query params → Request body → Response(s) per status code → Errors.
   Include the `operationId` (= handler function name) so the client can name its methods
   consistently with the server.

6. **Add a short "Client implementation notes"** section for anything a client would otherwise get
   wrong on the first try: date/time formats, the tri-state PATCH fields (list them again by
   endpoint), any non-JSON endpoint.

## Rust → JSON wire-format quick reference

| Rust type | JSON shape | Note |
|---|---|---|
| `Uuid` | string | standard UUID string |
| `NaiveDate` | string | `"YYYY-MM-DD"` |
| `NaiveDateTime` | string | ISO 8601, no UTC offset |
| `Option<T>` (no serde attr) | `T \| null`, field always present | |
| `Option<T>` + `skip_serializing_if = "Option::is_none"` | field **omitted** when `None` (response side only) | |
| `Option<Option<T>>` + `double_option` | tri-state, see "Fill the gaps" step above | request side only |
| field + `#[serde(skip_serializing)]` | never appears in the response, even though the Rust struct has it | e.g. `user_id` on `PlannerEntry` |
| `Vec<T>` | JSON array | |

## Common Mistakes

- Treating every path's `security(("token" = []))` block as proof of *which* permission is needed —
  it only proves the bearer scheme is used; the actual `Permission` variant is only in the `#[protect]`
  macro.
- Assuming an error response has a JSON body because `utoipa::path` gives it a description — check
  `IntoResponse` for the real answer.
- Missing that a handler has no `#[protect]` at all (unauthenticated route).
- Dropping the tri-state PATCH semantics down to a plain optional field.
- Manually re-deriving schemas from the Rust structs instead of generating OpenAPI first — manual
  transcription misses `serde(rename)`, flattening, and enum tagging.
