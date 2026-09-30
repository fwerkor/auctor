use anyhow::{Context, bail};
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{env, fs, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};

pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await
        .context("connect PostgreSQL")?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    Ok(pool)
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    if password.len() < 12 {
        bail!("password must be at least 12 characters");
    }
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow::anyhow!("password hashing failed: {err}"))?
        .to_string())
}

pub async fn bootstrap_admin(pool: &PgPool) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(pool)
        .await?;
    if count != 0 {
        return Ok(());
    }

    let username = match env::var("AUCTOR_BOOTSTRAP_USERNAME") {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };
    let email = env::var("AUCTOR_BOOTSTRAP_EMAIL")
        .context("AUCTOR_BOOTSTRAP_EMAIL is required for bootstrap")?;
    let password = env::var("AUCTOR_BOOTSTRAP_PASSWORD")
        .context("AUCTOR_BOOTSTRAP_PASSWORD is required for bootstrap")?;
    let display_name =
        env::var("AUCTOR_BOOTSTRAP_DISPLAY_NAME").unwrap_or_else(|_| username.clone());
    let password_hash = hash_password(&password)?;

    let mut tx = pool.begin().await?;
    let user_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO users(username,email,display_name,password_hash,email_verified_at) VALUES ($1,$2,$3,$4,now()) RETURNING id",
    )
    .bind(&username)
    .bind(email.trim().to_lowercase())
    .bind(&display_name)
    .bind(password_hash)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO user_roles(user_id, role_id)
         SELECT $1, id FROM roles WHERE name IN ('user','platform-admin')",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES ($1,'bootstrap_admin','user',$2,'{}'::jsonb)",
    )
    .bind(user_id)
    .bind(user_id.to_string())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn ensure_setup_token(pool: &PgPool) -> anyhow::Result<(Option<String>, PathBuf)> {
    let path = PathBuf::from(
        env::var("AUCTOR_SETUP_TOKEN_FILE")
            .unwrap_or_else(|_| "/var/lib/auctor/setup-token".to_owned()),
    );

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        let _ = fs::remove_file(&path);
        return Ok((None, path));
    }

    if let Ok(existing) = fs::read_to_string(&path) {
        let token = existing.trim().to_owned();
        if !token.is_empty() {
            return Ok((Some(token), path));
        }
    }

    let mut raw = [0u8; 32];
    rand::rng().fill_bytes(&mut raw);
    let token = URL_SAFE_NO_PAD.encode(raw);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(&path)?;
    writeln!(file, "{token}")?;

    Ok((Some(token), path))
}
