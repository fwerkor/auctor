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

## Registration and email verification

Self-service registration is disabled by default. Administrators can enable it under
**Settings → Registration and email** and can independently require new accounts to
verify their email address before signing in.

When verification is required, configure SMTP in the same settings section:

- SMTP host and port;
- STARTTLS, implicit TLS, or (only for a trusted local relay) unencrypted SMTP;
- optional SMTP authentication username and password;
- From address and display name.

The SMTP password is write-only through the administrator API: the browser only learns
whether a password is configured. Use **Send test email** after changing the transport.

Verification codes expire after ten minutes, are attempt-limited, and are stored using
Argon2 rather than in plaintext. Changing an existing account email always requires the
current password and a verification code sent to the new address, regardless of whether
registration verification is enabled.

## External account sign-in

Auctor can link external identities to existing global users. External providers are
login credentials only: an OAuth callback never creates an Auctor user and never links
by provider username or email. A user must first sign in with an existing Auctor account,
then explicitly bind the provider under **My account → Connected accounts**.

GitHub is supported through a GitHub OAuth App. Configure it under
**Settings → Registration and email → GitHub sign-in** by entering the OAuth Client ID
and Client Secret and enabling GitHub sign-in. Set the OAuth App callback URL to the
callback URL shown there:

`https://<your-auctor-host>/api/auth/external/github/callback`

The host is derived from Auctor's canonical site URL. GitHub access tokens are used only
for the callback exchange and `/user` lookup and are not persisted. Auctor stores the
stable GitHub numeric user ID plus display metadata. An unbound GitHub identity is
rejected and directed to sign in with an existing Auctor account first.

## WordPress SSO

The official plugin lives under integrations/wordpress/auctor-sso.

It uses Authorization Code + PKCE and deliberately stores no client secret. Register the
WordPress callback URI as an Auctor web application, then configure the plugin with the Auctor
public URL and client ID.

Existing WordPress users are linked by Auctor subject ID after the first successful login, with
username/email used only to find an existing account during that initial link. New users can be
created with the site's normal default role. The plugin never promotes a user to WordPress
administrator based on an Auctor role.
