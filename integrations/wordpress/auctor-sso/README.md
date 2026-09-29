# Auctor SSO for WordPress

Official lightweight WordPress integration for Auctor.

- OAuth 2.0 Authorization Code flow
- PKCE S256 required
- no client secret stored in WordPress
- stable account binding using the Auctor subject identifier
- existing username/email accounts are linked on first successful sign-in
- new users are optional and receive the site's default WordPress role
- emergency local WordPress login remains available with ?auctor_local=1

Register this callback URL in Auctor:

    https://your-wordpress-site.example/?auth=auctor

Then configure Settings > Auctor SSO with the Auctor public URL and client ID.
