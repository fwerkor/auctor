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

## Status

Auctor is pre-alpha. It is not yet suitable for production authentication traffic.

## Development

    cargo test --workspace
    cargo run -p auctor-server

Then:

    curl http://127.0.0.1:8080/health

## License

Apache-2.0.
