# Security policy

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub's private vulnerability reporting: <https://github.com/VikingTheDev/vikingthedev-platform/security/advisories/new>. Don't open a public issue.

Include what you found, how to reproduce it, and the impact you expect. You'll get an acknowledgement within 7 days. This is a personal project maintained by one person, so fixes are best-effort, and there's no bug bounty.

## Scope

- This repository.
- `vikingthe.dev` and the apps on its subdomains that are built from this repository.
- The admin hosts on `vikingthe.dev` subdomains, for access-control bypass only, including misconfiguration of the Cloudflare layer in front of them.

Out of scope:

- denial-of-service and volumetric attacks;
- findings that need physical access or a compromised device;
- reports produced only by automated scanners, without a demonstrated impact;
- third-party services (Cloudflare, GitHub, OVH, Proton) — report those to their owners.

## Testing

Test only against your own account and data. Don't access other users' data, degrade the service, or run automated scans above a few requests per second.

Good-faith research that follows these rules won't be pursued legally.
