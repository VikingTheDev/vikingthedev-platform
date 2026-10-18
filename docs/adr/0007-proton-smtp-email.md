# ADR 0007: Transactional email through Proton SMTP

- **Status:** Proposed. Pending evidence (D2).
- **Date:** 2026-10-18
- **Decider:** Solo maintainer

## Context

Contact-form notifications and auth emails (ADR 0004) need outbound mail. The maintainer wants no paid services and already has custom-domain addresses on a Proton Unlimited plan, which includes SMTP submission. SPF, DKIM (three `_domainkey` CNAMEs) and DMARC (`p=quarantine`) are already published for `vikingthe.dev`.

DNS hygiene (ADR 0001) forbids any `vikingthe.dev` host that serves HTTP from pointing at a third party. Many transactional providers ask for click-tracking or return-path CNAMEs, which would break that rule.

## Decision (proposed)

- **Transport:** send through Proton SMTP submission (port 587, STARTTLS), from a dedicated address such as `noreply@vikingthe.dev`, using an SMTP token.
- **Client:** the `lettre` crate. The dependency is justified in the commit that adds it.
- **The token:**
  - created by the human;
  - stored only as a Coolify environment variable;
  - never logged;
  - absent from the CI stack, which sends to Mailpit instead.
- **DNS:** no new records. DNS hygiene holds trivially: no tracking or bounce subdomains.

## Unverified, with fallbacks

| Item | Fallback |
|---|---|
| Proton's limits and terms allow low-volume automated mail | A free transactional tier, with tracking off and only DKIM and SPF records, never an HTTP-serving CNAME |
| OVH allows outbound 587 from the VPS | Port 465 (implicit TLS) |

## Evidence that would settle it

A test email sent from the VPS reaches an external inbox, with SPF, DKIM and DMARC all passing in its headers.
