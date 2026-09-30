# Auctor SSO for Nextcloud

Lightweight Nextcloud integration for Auctor.

It uses OAuth 2.0 Authorization Code with PKCE S256 and does not store a client secret in Nextcloud.

## Features

- Auctor sign-in entry on the Nextcloud login page.
- Stable account binding using the Auctor subject identifier.
- Existing accounts are linked by username first, then email.
- Optional automatic creation of Nextcloud users.
- Optional SSO-only Web login.
- Nextcloud app passwords, WebDAV, desktop/mobile clients, and API token authentication remain available in SSO-only mode.
- Configurable Auctor URL, client ID, OAuth scopes, and button label.

## Compatibility

The integration is currently tested with Nextcloud 31. The app metadata intentionally limits installation to Nextcloud 31 until newer versions are tested.

## Install

Copy this directory into the Nextcloud custom apps directory as auctor_sso, then enable it:

    php occ app:enable auctor_sso

For the official Docker/OCI image the directory is normally:

    /var/www/html/custom_apps/auctor_sso

## Register the Auctor application

The callback URL is generated from the public Nextcloud URL:

    https://cloud.example.com/apps/auctor_sso/callback

Create an active Auctor Web application using that exact redirect URI. A client secret is not required because the integration uses PKCE.

## Configure Nextcloud

Required:

    php occ config:app:set auctor_sso issuer --value=https://account.example.com
    php occ config:app:set auctor_sso client_id --value=your-nextcloud-client-id

Optional:

    php occ config:app:set auctor_sso scope --value="profile email groups roles"
    php occ config:app:set auctor_sso login_label --value="Sign in with Auctor"
    php occ config:app:set auctor_sso auto_create_users --type=boolean --value=true
    php occ config:app:set auctor_sso sso_only --type=boolean --value=false

Defaults:

- scope: profile email groups roles
- login_label: Sign in with Auctor
- auto_create_users: true
- sso_only: false

## SSO-only Web login

To make Auctor the only interactive Web login method:

    php occ config:app:set auctor_sso sso_only --type=boolean --value=true
    php occ config:system:set hide_login_form --type=boolean --value=true

The sso_only setting rejects direct POST /login password authentication. The hide_login_form system setting hides Nextcloud's password fields from the login page.

Before disabling the Auctor app on an SSO-only installation, restore the local login form:

    php occ config:system:set hide_login_form --type=boolean --value=false

This does not disable Nextcloud app passwords or token-based access, so WebDAV and official desktop/mobile clients can continue to use application credentials.

## User mapping

After a successful Auctor authorization the integration resolves the Nextcloud user in this order:

1. Existing auctor_sub binding.
2. Exact Nextcloud username matching Auctor preferred_username.
3. Exact email match.
4. Create a new Nextcloud user when auto_create_users=true.

The successful binding is stored as the Nextcloud user preference auctor_sso/auctor_sub.

## License

Apache-2.0, matching the Auctor repository.
