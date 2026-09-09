---
name: keeping-routes-openapi-in-sync
description: Use after adding, removing, renaming, or changing the method/path of any handler under crates/hikari-server/src/routes/api — before treating the change as complete.
---

# Keeping Routes and OpenAPI in Sync

## Overview

Every route handler in this codebase is described in three separate places that must agree:

1. The `#[utoipa::path(method, path = "...")]` annotation on the handler.
2. The actual `.route(...)` mount in that module's `create_router()`.
3. The `paths(...)` list in `crates/hikari-server/src/routes/swagger.rs`.

These three drift independently. `cargo test` do not catch such errors: the existing swagger tests only check that the OpenAPI JSON parses
and that operationIds are unique, not that a declared path matches where the handler is mounted.

**Tests passing does not mean the spec is accurate. You must check the router mount by hand.**

## Procedure

For every handler you touched (added, removed, renamed, or path/method changed):

1. Diff the handler's `#[utoipa::path]` `method` + `path` against its actual `.route(...)` call in
   `create_router()` for that module — full path including the router's mount prefix. They must be
   character-for-character identical (mind path params: `{id}` in both).
   ```
   rg -A1 'path = "' crates/hikari-server/src/routes/api/v0/<feature>*.rs
   rg '\.route\(' crates/hikari-server/src/routes/api/v0/<feature>*.rs
   ```
   If they disagree, the router mount is the truth (it's what the client actually hits) — fix the
   `#[utoipa::path]` annotation, don't silently keep both and pick one to believe.

2. Confirm `swagger.rs`'s `ApiDoc` `paths(...)` list has exactly one entry per annotated handler in
   the feature, using its full module path (e.g. `api::v0::planner::create_planner_entries`). Add
   entries for new handlers; remove entries for deleted ones (a stale reference to a renamed/removed
   function fails to compile, but a stale reference to a function that still exists but changed
   purpose will not).
   ```
   rg 'api::v0::<feature>::' crates/hikari-server/src/routes/swagger.rs
   ```

3. Run `cargo build -p hikari-server` (catches stale/missing swagger.rs references) and
   `cargo test -p hikari-server routes::swagger` (catches malformed JSON and duplicate operationIds —
   but remember, not path/mount drift; that's step 1's job, not this one's).

## Rationalization Table

| Excuse | Reality |
|---|---|
| "I only changed the handler body, not the route" | Check anyway — this is about catching pre-existing drift too, not just new drift you introduce. |
| "The tests pass" | The tests check JSON validity and operationId uniqueness. They do not check path/mount agreement. Passing tests prove nothing about this. |
| "It's just an internal endpoint" | The client-facing `exporting-feature-api` skill trusts this spec is accurate; every undetected drift becomes a wrong client implementation. |
| "I'll fix the doc later" | The annotation and the router are one diff apart right now. Fix it in the same change. |

## Red Flags — check the router mount before moving on

- You changed a `.route(...)` call without touching the corresponding `#[utoipa::path]`.
- You changed a `path =` or method in `#[utoipa::path]` without checking `create_router()`.
- You added a new handler and haven't added it to `swagger.rs`.
- You're relying on "the build succeeded" as proof the spec is correct — it isn't for this class of bug.
