# Production deployment

Auctor is designed to run as an unprivileged system service with PostgreSQL.

Recommended layout:

- /opt/auctor/auctor-server — release binary
- /opt/auctor/web — built web assets
- /etc/auctor/auctor.env — root-owned runtime configuration
- /var/lib/auctor — service-owned writable state if future features need local state
- PostgreSQL — local or external database

The service should sit behind a TLS reverse proxy. AUCTOR_SECURE_COOKIES=true
is required for public HTTPS deployments.

On an empty database, Auctor creates a one-time setup token at
`/var/lib/auctor/setup-token` with mode `0600`. Open the web UI, enter that token,
and choose the username, email, display name, and password for the first platform
administrator. After the administrator is created, Auctor deletes the token file
and disables first-run setup because the user directory is no longer empty.

For automated deployments, the legacy `AUCTOR_BOOTSTRAP_*` environment variables
remain available, but interactive setup is the preferred path because no account
password needs to be stored in a service environment file.

Do not expose a pre-alpha Auctor instance as the authoritative identity
provider for critical applications until OAuth/OIDC conformance, MFA, recovery,
rate limiting, key rotation, and security review are complete.
