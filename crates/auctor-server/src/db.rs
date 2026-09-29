use anyhow::{Context, bail};
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;

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
        "INSERT INTO users(username,email,display_name,password_hash) VALUES ($1,$2,$3,$4) RETURNING id",
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
