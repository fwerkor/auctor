ALTER TABLE email_verification_codes
    DROP CONSTRAINT IF EXISTS email_verification_codes_purpose_check;

ALTER TABLE email_verification_codes
    ADD CONSTRAINT email_verification_codes_purpose_check
    CHECK (purpose IN ('registration', 'email_change', 'password_reset'));
