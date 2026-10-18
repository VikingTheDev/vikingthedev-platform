# ADR 0004: Authentication method and access model

- **Status:** Proposed. Awaiting the maintainer's answers to D1a–D1g and D18.
- **Date:** 2026-10-18
- **Decider:** Solo maintainer

## Context

The maintainer wants user accounts (2026-10-18):

- **Methods:** email and password, or OAuth2 through GitHub or Discord. A magic link would be a nice extra.
- **Access:** gated apps such as the file drop must not be open to everyone.
- **Management:** an `admin.vikingthe.dev` app manages users and access.

The session model is fixed by ADR 0001. This ADR covers how a user proves who they are before a session is issued, and what that user may access.

## Options

| Option | Pros | Cons |
|---|---|---|
| Email and password (Argon2id) | Works for everyone. No third party | Credential stuffing, reset flows, CPU per hash, more ASVS surface |
| OAuth: GitHub and Discord | No password to store. Friends already have these accounts | A client secret per provider. A third party controls the login. Risks from account linking |
| Magic link | No stored credential | Each login depends on email delivery and on how secure the user's mailbox is |
| Passkeys | Phishing-resistant | Not requested. Can be added later |

## Decision (proposed)

**Authentication**

- **Email and password first.** Hashed with Argon2id (OWASP minimum parameters, inside `spawn_blocking`). Passwords need at least 12 characters, with no composition rules, and are checked against a bundled list of breached passwords.
- **Sign-up creates nothing usable until the email is verified.** This prevents pre-account hijacking.
  - A sign-up stores a *pending* record (email, password hash, display name) keyed by the verification token's hash, in Valkey for 24 hours.
  - Each new sign-up for the same email writes its own pending record, so an attacker's earlier password never sticks.
  - Verification creates the account from the record that token belongs to, and requires the password entered at sign-up, so a victim who clicks an attacker's link doesn't get an account the attacker controls.
  - A sign-up for an email that already has an account sends that owner a "someone tried to sign up" email instead.
- **Login** returns the same 401 for an unknown email, a wrong password or a disabled account, with a dummy hash on unknown emails so the timing matches.
- **Emails are sent off the request path,** in a background task, so response times don't reveal whether an account exists.
- **Then OAuth for GitHub and Discord.** Authorization code flow with PKCE and `state`.
  - `state` is bound to the browser: `/start` sets a short-lived `HttpOnly` pre-auth cookie that the callback must match. This prevents login CSRF.
  - There is one registered callback host, the apex.
  - The return path is either a relative path or an allowlisted `https://*.vikingthe.dev` URL, so there's no open redirect.
  - Identities are linked only from a logged-in session, never automatically by matching email.
  - If an OAuth login brings a verified email that already belongs to another account, the response is generic: "log in with your existing method, then link this provider". It's the same message whether or not the address exists.
- **The magic link exists as the password reset.** A single-use token of 256 bits from `OsRng`, stored as a SHA-256 hash in Valkey, valid for 15 minutes. It can later also be offered as a login method.
- **Every login, reset or link** issues a new session token.
- **Responses** never reveal whether an account exists.
- **Rate limits** apply per IP and per account.

**Authorization**

- **Open sign-up.** A `role` (`user` or `admin`) plus per-app grants in `app_grants`, checked in the domain layer and denied by default.
- **Gated apps** require a grant, issued from `admin.vikingthe.dev`.
- **Admin writes** (any non-GET request) need the admin role, a login within the last 15 minutes and an audit record.
  - The session record gains `last_auth_at`, an amendment to the ADR 0001 record shape.
  - Every `/api/admin/*` request must arrive on `Host: admin.vikingthe.dev` with a valid Cloudflare Access JWT. That makes Access a real second factor, until TOTP for admins exists. Only builds with the `e2e` feature accept a throwaway test signing key; production always verifies against Cloudflare's team certificates.

## Evidence that would settle it

- The maintainer's answers to D1a–D1g and D18 in `docs/decisions.md`.
- An Argon2id timing measurement on the VPS (under about 100 ms).
- A test against a mock OAuth provider in the CI stack (ADR 0002).

## Consequences

To be completed on acceptance.
