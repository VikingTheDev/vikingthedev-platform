# Capacity

- **Measured:** 2026-10-18, by the maintainer over ssh.
- **Rule:** every Coolify resource gets a memory limit (D13). Limits below are hard caps. "Expected" is the typical resident memory.

## Measured VPS

| Item | Value |
|---|---|
| Provider | OVH VPS |
| CPU | 8 vCPU, AMD EPYC-Milan, 1 thread per core. Steal was 0% over a 5-second `vmstat` sample. |
| Memory | 22.9 GiB visible (24 GB nominal). 1.5 GiB in use, 21 GiB available. |
| Swap | **None** (see D13) |
| Disk | One filesystem, 193 GB, with 184 GB free |
| Disk performance | About 1,700 synchronous 8 KiB writes per second (14 MB/s with `dd oflag=dsync`). fio isn't installed, so random IOPS are unmeasured |
| Network | 3 Gb/s (stated, not measured) |
| OS | Ubuntu 26.04.1 LTS, kernel 7.0.0-27 |
| Docker | 29.8.2 |
| Coolify | 4.4.6 |
| Traefik | v3.7 (`coolify-proxy`) |
| Ingress | Cloudflare Tunnel: `cloudflared` runs on the host as the `ubuntu` user (D9), with four tunnels |

### Running now

| Container | Memory | CPU at sample |
|---|---|---|
| coolify | 472 MiB | 57% of one core (a spike? check Beszel's history) |
| coolify-db | 29 MiB | 1% |
| coolify-redis | 7 MiB | 1% |
| coolify-proxy (Traefik) | 20 MiB | <1% |
| coolify-sentinel | 7 MiB | <1% |
| beszel + beszel-agent | 23 MiB | <1% |
| **Total** | **~560 MiB** | |

## Plan: production, with a future staging

| Service | Expected memory | Limit | CPU (expected / burst) | Notes |
|---|---|---|---|---|
| Coolify stack + Traefik | 0.6 GiB | — (not ours to limit) | 0.2 / 1 | Measured above |
| Beszel | 0.03 GiB | 128 MiB | ~0 | |
| cloudflared | 0.05 GiB | — (host service) | 0.1 / 0.5 | Estimate |
| API (axum) | 0.05–0.1 GiB | 512 MiB | 0.2 / 2 | Each Argon2id hash takes about 19 MiB and one core for under 100 ms (D1f). Rate limits cap concurrent hashes |
| PostgreSQL | 1.2 GiB | 2 GiB | 0.2 / 2 | `shared_buffers=512MB`. The OS page cache gives more |
| Valkey | 0.05 GiB | 512 MiB | ~0 / 0.5 | `maxmemory 256mb` with `maxmemory-policy noeviction`, because sessions and rate-limit counters are state that must never be silently evicted. Beszel alerts at 80% of `maxmemory`. The limit leaves room for the fork during AOF rewrites |
| homepage (Astro, node) | 0.1 GiB | 256 MiB | ~0 / 1 | Mostly served from the edge cache |
| canary | 0.08 GiB | 192 MiB | ~0 | |
| 5 apps × Astro | 0.5 GiB | 5 × 256 MiB | ~0 / 1 each | |
| Distributed-systems demo (extra nodes) | 0.3 GiB | 1 GiB, all nodes combined | 0.5 / 2 | Only if the live-cluster version of Quorum is built (`apps.md`) |
| Nightly `pg_dump` | brief | — | 1 core for minutes | Scheduled at night |
| **Production total** | **~3.1 GiB** | **~5.8 GiB of limits, plus ~0.65 GiB unlimited (Coolify stack, cloudflared)** | **~1.5 / bursts up to all 8** | |
| Staging (future: API, PostgreSQL, Valkey and apps) | ~1.5 GiB | ~3.5 GiB | ~0.5 | Only if it shares the VPS; it could also live on a small second VPS |

## Headroom

| | Memory |
|---|---|
| Visible | 22.9 GiB |
| Production limits, all at their caps | −5.8 GiB |
| Coolify stack and cloudflared (no limits) | −0.65 GiB |
| Future staging limits | −3.5 GiB |
| OS and Docker | −1 GiB |
| **Left, even with every container at its cap** | **~12 GiB** |

- **Memory:** enough for roughly 45 more Astro apps at their caps, or a second PostgreSQL. In practice the leftover RAM becomes page cache for PostgreSQL, which helps reads.
- **CPU:** expected load is under 2 of 8 cores. The limit is bursts: CI never builds on this machine (ADR 0001), so the bursts come from `pg_dump`, `oha` runs and the demo. Run `oha` and backups at different times.
- **Disk:** 184 GB free. Daily `pg_dump`s with 7 kept are negligible for years. User files go to R2, not this disk.
- **Bandwidth:** not a constraint. Cloudflare serves static assets, and uploads and downloads go straight to R2 through presigned URLs.
- **The real limit is the single point of failure,** which the maintainer accepts.

## Disk

About 1,700 synchronous 8 KiB writes per second is above the 1,000 a second where PostgreSQL commit latency would become a concern. Keep the defaults (`synchronous_commit=on`), and batch the RUM inserts anyway. Random-read IOPS (fio) are unmeasured. Measure them only if PostgreSQL read latency misses the budget in M2.
