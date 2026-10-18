# ADR 0005: PostgreSQL access with Diesel (diesel-async)

- **Status:** Proposed. Library chosen by the maintainer; the async variant is pending a spike.
- **Date:** 2026-10-18
- **Decider:** Solo maintainer

## Context

CLAUDE.md left the PostgreSQL library undecided. On 2026-10-18 the maintainer chose Diesel over `sqlx` and `tokio-postgres`. Two constraints still apply:

- The API runs on axum, which is async.
- The runtime image is distroless `cc`, which ships no `libpq`.

## Decision (proposed)

- **Library:** use `diesel-async` with the `postgres` backend, which runs on `tokio-postgres` and needs no libpq.
- **Pool:** a `bb8` or `deadpool` pool in `AppState`.
- **Schema:** `diesel` (sync) is used only for `schema.rs` generation, through the `diesel` CLI in development.
- **Migrations:** `diesel_migrations` through `AsyncConnectionWrapper`, run as D16 decides. The recommendation is at startup, under an advisory lock, expand-style only.
- **Error handling:** query errors are mapped to domain `thiserror` enums, never passed to clients.

**Rejected:** synchronous `diesel` with `deadpool-diesel`. It needs libpq in the runtime image, or static linking, and a `spawn_blocking` hop on every query.

## Evidence that would settle it

A 1-day spike in milestone 2, running three representative queries: a primary-key read, a cursor-paginated filtered list, and a two-table transaction. It passes if:

- the read p99 is at most 50 ms under `oha` (server-side timing);
- `ldd` on the binary shows no libpq;
- errors map cleanly to `AppError`.

## Consequences

- **Positive:** compile-time-checked queries in Rust, with no live database needed at build time, unlike `sqlx`'s macros. A pure-Rust driver.
- **Negative:**
  - `diesel-async` trails Diesel's releases;
  - complex dynamic queries need boxed queries;
  - a new dependency category, to be justified in the commit that adds it.
