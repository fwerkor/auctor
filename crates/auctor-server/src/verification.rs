use crate::email;
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

const MAX_ATTEMPTS: i16 = 5;

pub async fn issue_code(
    db: &PgPool,
    user_id: Uuid,
    email_address: &str,
    purpose: &str,
) -> anyhow::Result<()> {
    let code = format!("{:06}", rand::rng().random_range(0..1_000_000u32));
    let id = Uuid::new_v4();
    let code_hash = hash_code(&code)?;
    let expires_at = Utc::now() + Duration::minutes(10);

    let mut tx = db.begin().await?;
    sqlx::query(
        "UPDATE email_verification_codes
         SET consumed_at=COALESCE(consumed_at,now())
         WHERE user_id=$1 AND purpose=$2 AND consumed_at IS NULL",
    )
    .bind(user_id)
    .bind(purpose)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO email_verification_codes(id,user_id,email,purpose,code_hash,expires_at)
         VALUES($1,$2,lower($3),$4,$5,$6)",
    )
    .bind(id)
    .bind(user_id)
    .bind(email_address.trim())
    .bind(purpose)
    .bind(code_hash)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    if let Err(error) = email::send_verification_code(db, email_address, &code, purpose).await {
        let _ = sqlx::query("DELETE FROM email_verification_codes WHERE id=$1")
            .bind(id)
            .execute(db)
            .await;
        return Err(error);
    }
    Ok(())
}

pub async fn consume_code(
    db: &PgPool,
    user_id: Uuid,
    email_address: &str,
    purpose: &str,
    code: &str,
) -> Result<bool, sqlx::Error> {
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(false);
    }

    let mut tx = db.begin().await?;
    let row = sqlx::query_as::<_, (Uuid, Vec<u8>, i16)>(
        "SELECT id,code_hash,attempts
         FROM email_verification_codes
         WHERE user_id=$1 AND lower(email)=lower($2) AND purpose=$3
           AND consumed_at IS NULL AND expires_at>now()
         ORDER BY created_at DESC
         LIMIT 1
         FOR UPDATE",
    )
    .bind(user_id)
    .bind(email_address.trim())
    .bind(purpose)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((id, expected, attempts)) = row else {
        tx.rollback().await?;
        return Ok(false);
    };

    if attempts >= MAX_ATTEMPTS || !verify_code(id, code, &expected) {
        sqlx::query(
            "UPDATE email_verification_codes
             SET attempts=LEAST(attempts+1,20)
             WHERE id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(false);
    }

    sqlx::query("UPDATE email_verification_codes SET consumed_at=now() WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}

fn hash_code(code: &str) -> anyhow::Result<Vec<u8>> {
    let salt = SaltString::generate(&mut OsRng);
    let encoded = Argon2::default()
        .hash_password(code.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!("verification-code hashing failed: {error}"))?
        .to_string();
    Ok(encoded.into_bytes())
}

fn verify_code(id: Uuid, code: &str, expected: &[u8]) -> bool {
    if let Ok(encoded) = std::str::from_utf8(expected)
        && let Ok(parsed) = PasswordHash::new(encoded)
    {
        return Argon2::default()
            .verify_password(code.as_bytes(), &parsed)
            .is_ok();
    }

    // Backward compatibility for challenges created during the initial
    // deployment window before Argon2-backed challenge storage.
    if expected.len() == 32 {
        let mut hasher = Sha256::new();
        hasher.update(id.as_bytes());
        hasher.update(b":");
        hasher.update(code.as_bytes());
        return hasher.finalize().as_slice() == expected;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2_verification_code_round_trip() {
        let encoded = hash_code("482901").expect("hash code");
        assert!(verify_code(Uuid::new_v4(), "482901", &encoded));
        assert!(!verify_code(Uuid::new_v4(), "482902", &encoded));
    }

    #[test]
    fn legacy_sha256_challenge_is_still_accepted() {
        let id = Uuid::new_v4();
        let code = "123456";
        let mut hasher = Sha256::new();
        hasher.update(id.as_bytes());
        hasher.update(b":");
        hasher.update(code.as_bytes());
        let expected = hasher.finalize().to_vec();
        assert!(verify_code(id, code, &expected));
        assert!(!verify_code(id, "654321", &expected));
    }
}
