# Architecture

## Identity model

Auctor has one global user namespace.

Administrative access is represented by roles and permissions granted to normal users. There is no special built-in administrator identity class and no organization membership is required to become an OAuth/OIDC end user.

Initial domain entities:

- User
- Credential
- Session
- Application
- AuthorizationGrant
- Role
- Permission
- Group
- AuditEvent

Organizations or tenants may be added as optional authorization scopes. They must not become ownership containers for global user identities.

## Protocol roadmap

The initial standards target is OpenID Connect Core on OAuth 2.0:

1. Provider discovery
2. Authorization Code with PKCE
3. Token endpoint
4. JWKS
5. ID tokens
6. UserInfo
7. Refresh token rotation
8. Device Authorization Grant
9. Revocation
10. RP-initiated logout where interoperable

Protocol conformance is tested independently from the account/admin UI.

## Components

Browser and native clients call auctor-server. The server separates the identity domain model from the OAuth/OIDC protocol layer and persists state in PostgreSQL. The account portal and administration console are normal clients of server APIs and receive no implicit privilege merely for being first-party UIs.

## Branding

Auctor branding is runtime configuration stored in PostgreSQL rather than compiled into the
frontend. Administrators can change the site name, canonical public URL, and logo URL from the
Branding page. The default logo is served at /brand/auctor.svg.

The canonical site URL is also the issuer/base URL used by OAuth metadata, so production
deployments should set it to the public HTTPS origin.

## OAuth 2.0

Auctor currently provides OAuth 2.0 Authorization Code with mandatory PKCE S256 for public
clients.

Endpoints:

- GET /oauth/authorize
- POST /oauth/token
- GET /oauth/userinfo
- GET /.well-known/oauth-authorization-server

Authorization codes expire after five minutes and are single-use. Access tokens expire after
one hour. The database stores SHA-256 hashes of codes and tokens rather than their bearer values.

OpenID Connect ID tokens are not enabled yet. The OIDC discovery endpoint states this explicitly
instead of advertising incomplete conformance.

## Public-facing hardening

The application adds security headers directly, including CSP, frame denial, referrer policy,
content-type sniffing protection, permissions policy, COOP, and HSTS. Authentication, setup,
OAuth token, authorization, and general API traffic have per-client rate limits. API and OAuth
responses use no-store caching directives, and request bodies have a global size limit.
