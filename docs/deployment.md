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

## Public branding

After first sign-in, open Branding in the administrator console and set:

- the public site name;
- the canonical HTTPS site URL;
- a local or HTTPS logo URL.

The canonical site URL must be configured before OAuth metadata can be used.

## WordPress SSO

The official plugin lives under integrations/wordpress/auctor-sso.

It uses Authorization Code + PKCE and deliberately stores no client secret. Register the
WordPress callback URI as an Auctor web application, then configure the plugin with the Auctor
public URL and client ID.

Existing WordPress users are linked by Auctor subject ID after the first successful login, with
username/email used only to find an existing account during that initial link. New users can be
created with the site's normal default role. The plugin never promotes a user to WordPress
administrator based on an Auctor role.
