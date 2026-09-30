# Auctor

Auctor is an open-source identity and access platform for humans, applications, and infrastructure.

Auctor uses a global user model. A user keeps the same identity whether they are signing in to an OAuth/OIDC client, using the account portal, or exercising administrative permissions.

## Principles

- Global users are first-class identities.
- Administration is authorization, not a separate account type.
- Applications never own users.
- OAuth 2.0 and OpenID Connect are core protocol surfaces.
- Roles, groups, and permissions are independent building blocks.
- Organizations and tenants are optional authorization domains, not mandatory identity containers.
- Self-hosting is a primary deployment model.
- Privileged actions must be auditable.
- Secure defaults take precedence over configuration convenience.

## Workspace

- crates/auctor-core: protocol-independent identity and authorization model.
- crates/auctor-server: HTTP server and future OAuth/OIDC endpoints.
- docs/architecture.md: architecture and protocol roadmap.
- docs/security.md: security boundaries and non-goals.

## Current capabilities

- Global users with Argon2id password credentials and server-side revocable sessions.
- The same identity can be an ordinary application user and a platform administrator.
- Material-style responsive account center and administration console.
- User search, create/edit/disable, password reset, bulk actions, and audit history.
- Custom groups and roles with user membership and assignment management.
- Per-user and self-service session/device inspection and revocation.
- Application/client registration with generated client IDs and strict redirect URI policy.
- Gravatar avatars proxied through Auctor so browsers do not contact Gravatar directly.
- PostgreSQL persistence and a one-time secure first-run administrator setup flow.
- Administrator-controlled self-service registration, disabled by default.
- Optional mandatory registration email verification with expiring, attempt-limited Argon2-backed codes.
- Configurable SMTP transport with STARTTLS or implicit TLS and a built-in test-email action.
- Verified account email changes: current password plus a code delivered to the new address.

## Status

Auctor is pre-alpha. The identity/account and administration foundation is usable for development and evaluation, but OAuth 2.0/OpenID Connect token issuance, MFA/passkeys, account recovery, and protocol conformance work are still required before Auctor should be used as the authoritative identity provider for critical applications.

## Development

    cargo test --workspace
    cargo run -p auctor-server

Then:

    curl http://127.0.0.1:8080/health

## License

Apache-2.0.
