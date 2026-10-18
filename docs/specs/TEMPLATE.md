# Spec: <app name>

- **Status:** Draft | Ready | Implemented
- **Host:** `<name>.vikingthe.dev` (or `vikingthe.dev/<path>`, see ADR 0001)
- **Related ADRs:** 0001

## Purpose

One or two sentences: what this app is for and why it exists.

## Users

Who uses it, and whether they must be logged in.

## User stories

- As a <user>, I can <action>, so that <outcome>.

Each story gets acceptance criteria as checkable statements:

- [ ] <observable behavior>

## Pages and routes

| Route | Purpose | Rendering (prerendered / on demand / server island) | Auth required |
|---|---|---|---|

## Data

The entities this app owns or reads, their fields, and their relations. Note which data is personal.

## API

| Method | Path | Request | Response | Auth | Notes |
|---|---|---|---|---|---|

## Auth and security

Who can access what (authorization rules per resource), rate limits, uploads, and anything sensitive.

## Performance

Any deviation from the default budgets in CLAUDE.md, with a reason. List the route that's expected to be the largest JS payload.

## Out of scope

What this app deliberately does not do (yet).

## Open questions

Must be empty before implementation starts.

- <question>
