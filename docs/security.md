# Security model

Auctor is security-sensitive infrastructure. Features that affect credentials, tokens, cryptographic keys, sessions, or authorization are security boundaries.

## Initial invariants

- An administrator is a normal user with explicit privileges.
- OAuth clients cannot grant themselves permissions.
- Redirect URIs are exact-match by default.
- Authorization Code flow requires PKCE for public clients.
- Refresh tokens are rotated and replay-detectable.
- Password storage will use a memory-hard KDF.
- Signing keys are never stored in source code.
- Privileged mutations emit immutable audit events.
- Authentication and authorization decisions remain server-side.

## Pre-alpha limitations

The current repository does not yet implement credential verification or issue OAuth/OIDC tokens. The discovery route is deliberately marked not configured. Do not place real users behind this build.
