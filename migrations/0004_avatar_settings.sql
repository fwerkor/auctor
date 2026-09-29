ALTER TABLE site_settings
    ADD COLUMN avatar_source_template varchar(2048) NOT NULL
        DEFAULT 'https://www.gravatar.com/avatar/{email_md5}?s={size}&d=404',
    ADD COLUMN avatar_delivery varchar(16) NOT NULL
        DEFAULT 'direct'
        CHECK (avatar_delivery IN ('direct', 'proxy'));
