# Contributing to Auctor

Auctor welcomes focused contributions to the identity model, administration experience,
protocol implementation, security hardening, documentation, and tests.

## Development checks

Before opening a pull request, run:

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

Then build the web application:

    cd web
    pnpm install --frozen-lockfile
    pnpm build

CI runs the same Rust and web checks.

## Repository layout

- `crates/auctor-core` — protocol-independent identity and authorization model.
- `crates/auctor-server` — HTTP APIs, authentication, administration, and protocol endpoints.
- `web` — React/TypeScript account center and administration console.
- `migrations` — ordered PostgreSQL migrations.
- `docs` — architecture, security, and deployment documentation.
- `deploy` — production service templates.

## Database changes

Database migrations are append-only once merged to `main`. Do not rewrite an existing
migration that may already have been applied to a deployment. Add a new numbered migration
instead.

## Identity model invariants

Contributions should preserve these design rules unless an architecture discussion reaches
a different conclusion:

- users are global identities;
- administrator access is authorization granted to a normal user;
- applications do not own users;
- groups and roles are independent from identity ownership;
- privileged mutations must be auditable;
- authentication and authorization decisions remain server-side.

## Security-sensitive changes

Changes involving passwords, sessions, cookies, OAuth/OIDC, cryptography, setup/bootstrap,
or authorization boundaries require tests for both the allowed and denied path. Avoid
introducing custom cryptographic primitives when a well-reviewed standard implementation
exists.

Please follow `SECURITY.md` for vulnerability reports rather than opening a public issue.
