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

Bootstrap variables are intentionally not part of the persistent environment
example. They should be supplied only for the first start on an empty database:

- AUCTOR_BOOTSTRAP_USERNAME
- AUCTOR_BOOTSTRAP_EMAIL
- AUCTOR_BOOTSTRAP_DISPLAY_NAME
- AUCTOR_BOOTSTRAP_PASSWORD

After the first user exists, remove bootstrap variables and restart the service.

Do not expose a pre-alpha Auctor instance as the authoritative identity
provider for critical applications until OAuth/OIDC conformance, MFA, recovery,
rate limiting, key rotation, and security review are complete.
