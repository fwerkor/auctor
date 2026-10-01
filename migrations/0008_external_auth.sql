ALTER TABLE site_settings
    ADD COLUMN github_oauth_enabled boolean NOT NULL DEFAULT false,
    ADD COLUMN github_oauth_client_id varchar(255) NOT NULL DEFAULT '',
    ADD COLUMN github_oauth_client_secret text NOT NULL DEFAULT '';

CREATE TABLE external_identities (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider varchar(64) NOT NULL,
    subject varchar(255) NOT NULL,
    login varchar(255) NOT NULL DEFAULT '',
    display_name varchar(255),
    avatar_url text,
    profile_url text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (provider, subject),
    UNIQUE (user_id, provider)
);

CREATE INDEX external_identities_user_idx
    ON external_identities(user_id, provider);

CREATE TABLE external_auth_flows (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    state_hash bytea NOT NULL UNIQUE,
    provider varchar(64) NOT NULL,
    intent varchar(16) NOT NULL CHECK (intent IN ('login', 'bind')),
    user_id uuid REFERENCES users(id) ON DELETE CASCADE,
    return_to text NOT NULL DEFAULT '/',
    expires_at timestamptz NOT NULL,
    used_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (
        (intent = 'login' AND user_id IS NULL)
        OR
        (intent = 'bind' AND user_id IS NOT NULL)
    )
);

CREATE INDEX external_auth_flows_active_idx
    ON external_auth_flows(expires_at)
    WHERE used_at IS NULL;
