# Security Policy

Auctor is identity and access infrastructure. Authentication, authorization, credentials,
sessions, cryptographic material, protocol handling, and administrator boundaries are
security-sensitive components.

## Reporting a vulnerability

Do not open a public GitHub issue for a vulnerability that could expose credentials,
sessions, signing keys, user data, or authorization boundaries.

Please report security issues privately to the repository owner through GitHub's private
vulnerability reporting feature when it is available for the repository. If that feature
is unavailable, contact the maintainer privately before publishing technical details.

Include, when possible:

- the affected commit or version;
- a minimal reproduction;
- the expected and observed authorization boundary;
- whether credentials, tokens, or user data can be exposed;
- any prerequisites or attacker capabilities.

## High-priority classes

Reports involving these areas are especially important:

- authentication bypass;
- privilege escalation or cross-user access;
- session fixation, theft, or revocation failures;
- unsafe password or credential handling;
- OAuth/OIDC redirect URI validation;
- token, signing-key, or secret disclosure;
- CSRF or cross-origin authorization issues;
- first-run setup takeover;
- audit-log integrity;
- SQL injection or unsafe deserialization.

## Pre-alpha status

Auctor is currently pre-alpha. OAuth 2.0/OpenID Connect token issuance, MFA/passkeys,
account recovery, rate limiting, key rotation, and formal protocol conformance are still
under development. Do not use the current build as the authoritative identity provider
for critical production systems.
