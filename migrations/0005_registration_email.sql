ALTER TABLE users
    DROP CONSTRAINT users_status_check,
    ADD CONSTRAINT users_status_check CHECK (status IN ('active', 'disabled', 'pending_email'));

ALTER TABLE users
    ADD COLUMN email_verified_at timestamptz;

UPDATE users
SET email_verified_at = COALESCE(email_verified_at, created_at)
WHERE status <> 'pending_email';

ALTER TABLE site_settings
    ADD COLUMN registration_enabled boolean NOT NULL DEFAULT false,
    ADD COLUMN registration_require_email_verification boolean NOT NULL DEFAULT false,
    ADD COLUMN smtp_host varchar(320) NOT NULL DEFAULT '',
    ADD COLUMN smtp_port integer NOT NULL DEFAULT 587 CHECK (smtp_port BETWEEN 1 AND 65535),
    ADD COLUMN smtp_security varchar(16) NOT NULL DEFAULT 'starttls'
        CHECK (smtp_security IN ('starttls', 'tls', 'none')),
    ADD COLUMN smtp_username varchar(320) NOT NULL DEFAULT '',
    ADD COLUMN smtp_password text NOT NULL DEFAULT '',
    ADD COLUMN smtp_from_email varchar(320) NOT NULL DEFAULT '',
    ADD COLUMN smtp_from_name varchar(160) NOT NULL DEFAULT 'Auctor';

CREATE TABLE email_verification_codes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid REFERENCES users(id) ON DELETE CASCADE,
    email varchar(320) NOT NULL,
    purpose varchar(32) NOT NULL CHECK (purpose IN ('registration', 'email_change')),
    code_hash bytea NOT NULL,
    expires_at timestamptz NOT NULL,
    attempts smallint NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 20),
    consumed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX email_verification_active_idx
    ON email_verification_codes (email, purpose, created_at DESC)
    WHERE consumed_at IS NULL;
