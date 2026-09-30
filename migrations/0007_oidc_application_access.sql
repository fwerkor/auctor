ALTER TABLE applications
    ADD COLUMN allowed_roles jsonb NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN allowed_groups jsonb NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE applications
    ADD CONSTRAINT applications_allowed_roles_array CHECK (jsonb_typeof(allowed_roles) = 'array'),
    ADD CONSTRAINT applications_allowed_groups_array CHECK (jsonb_typeof(allowed_groups) = 'array');

ALTER TABLE oauth_authorization_codes
    ADD COLUMN nonce text;

CREATE TABLE oauth_refresh_tokens (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash bytea NOT NULL UNIQUE,
    application_id uuid NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope text NOT NULL DEFAULT '',
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX oauth_refresh_tokens_user_idx
ON oauth_refresh_tokens(user_id, expires_at)
WHERE revoked_at IS NULL;
