# ADR 0003: Cloudflare Tunnel ingress and admin hosts

- **Status:** Accepted (2026-10-19)
- **Date:** 2026-10-18
- **Decider:** Solo maintainer

## Context

ADR 0001 assumes plain Cloudflare proxying to Traefik. The DNS zone shows something else: the apex and `*.vikingthe.dev` are CNAMEs to a Cloudflare Tunnel. `coolify.`, `beszel.` and `ssh.vikingthe.dev` go to three more tunnel IDs. All of these hosts receive the `Domain=vikingthe.dev` session cookie.

What a tunnel changes:

- **Client IP:** Traefik and the API see `cloudflared` as the peer. The real IP is only in `CF-Connecting-IP`.
- **TLS:** traffic from Cloudflare to the origin runs inside the tunnel. The hop from `cloudflared` to Traefik may be plain HTTP, which interacts with Coolify's HTTPS redirects.
- **Origin exposure:** with a tunnel, the origin needs no open inbound HTTP ports. If they are open anyway, a client can skip Cloudflare and forge `CF-Connecting-IP`.

## Decision

1. **One catch-all tunnel serves the public apps**, with the apex and wildcard pointing to Traefik. Coolify domains use the scheme that avoids redirect loops; milestone 1 step 14 settles which. The admin tools keep their own tunnels.
2. **The origin's inbound 80 and 443 are closed** to the internet (OVH's edge firewall; `ufw` doesn't cover Docker-published ports, see D20). Only `cloudflared`'s outbound connection carries traffic.
3. **The API takes the client IP from `CF-Connecting-IP`,** which is safe only because of point 2.
   - The rule applies only on the public `:8080` listener. `/api/health` is exempt, because Coolify's health checks, `oha` runs on the VPS and the CI stack arrive without Cloudflare.
   - The name of the trusted header is runtime config. In the CI stack, Traefik injects it.
   - A request that needs the IP (anything rate-limited) and lacks the header is rejected. Others proceed.
   - The value is parsed into a validated `ClientIp` type at the boundary. For IPv6, `ClientIp` exposes the /64 prefix, which is what rate limits key on, because one client usually controls a whole /64.
4. **Admin hosts** (`coolify.`, `beszel.`, `ssh.`):
   - Cloudflare Access goes in front of all three now (the maintainer agreed on 2026-10-18).
   - One exception: a Bypass rule on `coolify.vikingthe.dev/api/v1/deploy`, so CI's deploy webhook works. Coolify's deploy-only token still protects that path.
   - Our own `admin.vikingthe.dev` app also sits behind Access, if D19 is accepted. Access only guards requests that arrive on `admin.`, and `/api` is routed on every host. So the API itself must check `Host` and the `Cf-Access-Jwt-Assertion` on `/api/admin/*` (D1g).
   - They move to a dedicated admin domain when one is bought (never the staging domain; see D7 in `decisions.md`), so the session cookie never reaches them.
5. **The wildcard record stays.** It points at our own tunnel and Traefik, and unknown hosts get Traefik's 404, so nothing dangles. Retired apps still have their Traefik routes removed.

## Consequences

- **Positive:**
  - the origin IP isn't exposed;
  - there's no inbound attack surface besides SSH, which also goes through the tunnel;
  - TLS certificates are Cloudflare's job.
- **Negative:**
  - another component, `cloudflared`, on the request path, adding latency to TTFB and `oha` numbers that milestone 1 measures;
  - rate limiting depends on the firewall staying closed.

## Facts (D9, confirmed by the maintainer on 2026-10-18)

- `cloudflared` ran on the VPS as `root`, with one copy installed by hand outside apt.
- There are four tunnels:
  - `ssh.`;
  - `beszel.`;
  - `coolify.`;
  - a catch-all that sends the apex and every other `*.vikingthe.dev` to Coolify's Traefik.
- All other inbound traffic is blocked by OVH's edge firewall. It's the only layer in front of Docker-published ports and sshd, because those bypass `ufw` (D20). Points 1 and 2 of the decision already hold.
- The tunnels stay separate, so a leaked credential for one admin tunnel doesn't expose the public ingress.
- **Done in M0 (2026-10-18):**
  - all four tunnels run as an unprivileged `cloudflared` system user, under hardened systemd units, with the apt-packaged binary. Cloudflare's own packaged service runs as root, so it wasn't used;
  - tunnel credentials are readable only by that user, and the ssh tunnel's token is in a root-only environment file instead of the command line;
  - the ssh tunnel was recreated, which rotated its partly exposed token;
  - every tunnel forces `http2`, because QUIC over UDP fails through OVH's firewall. cloudflared's startup check still tests QUIC, but in the background; connections register within half a second.
