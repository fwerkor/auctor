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
