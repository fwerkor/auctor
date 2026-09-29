CREATE TABLE site_settings (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    site_name varchar(120) NOT NULL DEFAULT 'Auctor',
    site_url varchar(2048) NOT NULL DEFAULT '',
    logo_url varchar(2048) NOT NULL DEFAULT '/brand/auctor.svg',
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO site_settings(singleton) VALUES (true)
ON CONFLICT (singleton) DO NOTHING;

CREATE TABLE oauth_authorization_codes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code_hash bytea NOT NULL UNIQUE,
    application_id uuid NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    redirect_uri text NOT NULL,
    scope text NOT NULL DEFAULT '',
    code_challenge varchar(160) NOT NULL,
    code_challenge_method varchar(16) NOT NULL DEFAULT 'S256',
    expires_at timestamptz NOT NULL,
    used_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX oauth_codes_expiry_idx ON oauth_authorization_codes(expires_at)
WHERE used_at IS NULL;

CREATE TABLE oauth_access_tokens (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash bytea NOT NULL UNIQUE,
    application_id uuid NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope text NOT NULL DEFAULT '',
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX oauth_tokens_user_idx ON oauth_access_tokens(user_id, expires_at)
WHERE revoked_at IS NULL;

CREATE TABLE reserved_usernames (
    username varchar(64) PRIMARY KEY,
    note varchar(240) NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO reserved_usernames(username,note) VALUES
    ('admin','Administrative identity'),
    ('administrator','Administrative identity'),
    ('root','System identity'),
    ('system','System identity'),
    ('sysadmin','System identity'),
    ('operator','System identity'),
    ('owner','Platform identity'),
    ('support','Support identity'),
    ('security','Security identity'),
    ('webmaster','Infrastructure identity'),
    ('hostmaster','Infrastructure identity'),
    ('postmaster','Mail infrastructure identity'),
    ('abuse','Abuse contact identity'),
    ('api','Reserved service route'),
    ('auth','Reserved authentication route'),
    ('oauth','Reserved authentication route'),
    ('oidc','Reserved authentication route'),
    ('sso','Reserved authentication route'),
    ('login','Reserved authentication route'),
    ('logout','Reserved authentication route'),
    ('signin','Reserved authentication route'),
    ('signup','Reserved registration route'),
    ('register','Reserved registration route'),
    ('account','Reserved account route'),
    ('accounts','Reserved account route'),
    ('setup','Reserved setup route'),
    ('status','Reserved service route'),
    ('health','Reserved service route'),
    ('config','Reserved configuration route'),
    ('settings','Reserved configuration route'),
    ('dashboard','Reserved interface route'),
    ('console','Reserved interface route'),
    ('application','Reserved platform object'),
    ('applications','Reserved platform object'),
    ('service','Reserved platform object'),
    ('services','Reserved platform object'),
    ('session','Reserved platform object'),
    ('sessions','Reserved platform object'),
    ('token','Reserved authentication object'),
    ('tokens','Reserved authentication object')
ON CONFLICT (username) DO NOTHING;
