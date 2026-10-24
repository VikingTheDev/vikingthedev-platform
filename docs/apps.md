# App shortlist

- **Status:** Draft. No specs yet; each chosen app gets `docs/specs/<app>.md` from `TEMPLATE.md` before any code.
- **Source:** the maintainer's interests (distributed systems, a friends-only file share, daily puzzle games, small utilities).

## Scoring

Each factor is scored 1–5, and the score is their product (maximum 625).

| Factor | 5 means |
|---|---|
| **Value** | Strong portfolio signal, or something the maintainer uses weekly |
| **Fit** | Trivial on the measured VPS and free tiers (`capacity.md`) |
| **Effort** | Small: a couple of weekends |
| **Accounts** | Works with no account. A low score means it needs auth, invites and GDPR handling (M3) first |

| App (working name, host) | Value | Fit | Effort | Accounts | Score | Earliest milestone |
|---|---|---|---|---|---|---|
| Daily puzzles: **Summit** (`play.`) | 4 | 5 | 4 | 5 | **400** | After M5; no auth needed |
| CRDT live board: **Drift** (`drift.`) | 4 | 4 | 2 | 4 | **128** | After M5 |
| Consensus playground: **Quorum** (`quorum.`) | 5 | 3 | 2 | 4 | **120** | After M5 |
| Friends' paste and short links: **Relay** (`relay.`) | 3 | 5 | 4 | 2 | **120** | After M3 |
| Friends-only file drop: **Stash** (`stash.`) | 5 | 3 | 2 | 1 | **30** | After M3 |

The platform status page is a homepage section (`/api/status`), not an app. The terminal easter egg also lives on the homepage.

**Platform app, not scored:** `admin.vikingthe.dev` manages users and per-app access grants (D18, D19; invites only if D18 lands on invite-only). It's infrastructure the gated apps depend on, so it's built in M3, before Stash, from its own spec.

**Order (confirmed by the maintainer on 2026-10-18):**

1. **Summit**, because it needs no auth and can ship before M3.
2. **Stash**, despite its low score. It's the maintainer's own need, and M3 will have paid for the auth it needs.
3. **Quorum**, the portfolio piece.

Drift and Relay come after that, as and when they're wanted.

## Candidates

### Summit: daily topographic puzzle

**Job to be done:** when I have five minutes with my coffee, I want a short, fair daily puzzle I can share my result from, so I get a small win and something to compare with friends.

**Pitch:** each day shows the contour lines of one real mountain, with no labels. Players guess the peak in six tries, and each wrong guess reveals a hint: elevation band, region, distance and direction from the guess. The contours come offline from open elevation data (Kartverket's Norwegian data or Copernicus DEM; check the licence). They're rendered as static SVGs at build time, so every puzzle page is prerendered and cached at the edge. It fits the topographic design theme exactly. Streaks live in `localStorage`, with an optional sync to an account after M3. The result card is a text grid, so there's no image generation. Later it can host more daily games under `play.`.

### Quorum: consensus playground

**Job to be done:** when I'm explaining or learning how distributed consensus behaves under failure, I want to break a real cluster and watch it recover, so the algorithm makes intuitive sense.

**Pitch:** a live five-node Raft cluster, made of small Rust processes on the VPS, capped together at 1 GiB and 2 vCPU. Visitors can kill the leader, partition the network, add latency or drop messages, and watch elections, log replication and commits as they happen over SSE. This is the strongest signal of the maintainer's distributed-systems skill. Visitors share the cluster state, so the controls are rate-limited and the cluster resets on a timer. A fallback with Fit 5 is a deterministic simulation compiled to WASM and run in the browser: no server cost, but it must still meet the 75 KB JS budget, with the WASM measured separately. The spec decides between them.

### Drift: CRDT live board

**Job to be done:** when I'm collaborating with a friend in real time, or demonstrating eventual consistency, I want edits that merge without conflicts even offline, so collaboration just works and the mechanism is visible.

**Pitch:** a shared sticky-note board backed by a CRDT. An "offline" toggle and a simulated-latency slider let visitors watch replicas diverge and converge, with a side panel showing each replica's state. Rooms are anonymous with short random links. Persistence goes in PostgreSQL, and fan-out is SSE from the API. Anonymous rooms expire after 7 days, which keeps it within GDPR's minimisation principle.

### Relay: paste and short links for friends

**Job to be done:** when I want to send a friend a snippet or a long link, I want a private, fast paste or short link on my own domain, so I'm not relying on ad-filled services.

**Pitch:** gated by an access grant. Pastes with syntax highlighting done server-side (no client JS), expiry, and short links with a click count. It's small, useful every week, and a good first exercise of the M3 authorization rules (owner-only deletion, IDOR tests).

### Stash: friends-only file drop

**Job to be done:** when I need to share or stash files with friends, I want simple uploads and expiring links on my own platform, so I don't depend on a third-party service or pay for storage I don't use.

**Pitch:** users with a Stash grant upload straight to a private EU-jurisdiction R2 bucket through presigned PUTs, so files never pass through the VPS. They share through short-lived presigned download links. The server enforces per-user quotas, carved out of R2's 10 GB free tier (for example 1 GB per user and 8 users), plus a maximum size and allowed content types. Low on the score because it needs accounts, quotas and careful security (ASVS V5), but it's the maintainer's own need and the reason auth exists. Open questions for its spec are in D17.
