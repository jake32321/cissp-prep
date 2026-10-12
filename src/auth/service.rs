//! Account storage and password checks. Server only.

use std::sync::OnceLock;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use sqlx::SqlitePool;

use super::User;

pub const MIN_PASSWORD_LEN: usize = 12;
pub const MAX_PASSWORD_LEN: usize = 128;
const MAX_EMAIL_LEN: usize = 254;

/// Errors whose `Display` text is safe to show to the person signing in.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Enter a valid email address.")]
    InvalidEmail,
    #[error("Password must be {MIN_PASSWORD_LEN} to {MAX_PASSWORD_LEN} characters long.")]
    InvalidPassword,
    #[error("An account with that email already exists.")]
    EmailTaken,
    #[error("Sign-ups are closed.")]
    SignupsClosed,
    #[error("Incorrect email or password.")]
    InvalidCredentials,
    #[error("Something went wrong. Please try again.")]
    Database(#[from] sqlx::Error),
    #[error("Something went wrong. Please try again.")]
    Internal(String),
}

/// Trims and checks an email address. Deliberately loose: the real check is
/// whether mail arrives, which we don't send yet.
pub fn normalize_email(email: &str) -> Result<String, AuthError> {
    let email = email.trim();
    let valid = email.len() <= MAX_EMAIL_LEN
        && !email.contains(char::is_whitespace)
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        });
    if valid {
        Ok(email.to_string())
    } else {
        Err(AuthError::InvalidEmail)
    }
}

pub fn validate_password(password: &str) -> Result<(), AuthError> {
    let len = password.chars().count();
    if (MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&len) {
        Ok(())
    } else {
        Err(AuthError::InvalidPassword)
    }
}

/// Sign-ups are open when the config allows them, or when no account exists
/// yet so the owner can create theirs.
pub async fn signups_open(pool: &SqlitePool, allow_signups: bool) -> Result<bool, AuthError> {
    if allow_signups {
        return Ok(true);
    }
    let any_user: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users)")
        .fetch_one(pool)
        .await?;
    Ok(!any_user)
}

pub async fn create_user(
    pool: &SqlitePool,
    allow_signups: bool,
    email: &str,
    password: &str,
) -> Result<User, AuthError> {
    let email = normalize_email(email)?;
    validate_password(password)?;
    if !signups_open(pool, allow_signups).await? {
        return Err(AuthError::SignupsClosed);
    }

    let password_hash = hash_password(password.to_string()).await?;

    // Re-check inside the insert so two simultaneous "first account" sign-ups
    // can't both succeed while sign-ups are closed.
    let inserted = sqlx::query_as::<_, (i64, String)>(
        "INSERT INTO users (email, password_hash)
         SELECT ?1, ?2 WHERE ?3 OR NOT EXISTS (SELECT 1 FROM users)
         RETURNING id, email",
    )
    .bind(&email)
    .bind(&password_hash)
    .bind(allow_signups)
    .fetch_optional(pool)
    .await;

    match inserted {
        Ok(Some((id, email))) => Ok(User { id, email }),
        Ok(None) => Err(AuthError::SignupsClosed),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(AuthError::EmailTaken),
        Err(e) => Err(e.into()),
    }
}

pub async fn verify_credentials(
    pool: &SqlitePool,
    email: &str,
    password: &str,
) -> Result<User, AuthError> {
    let row = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, email, password_hash FROM users WHERE email = ?1",
    )
    .bind(email.trim())
    .fetch_optional(pool)
    .await?;

    // Always run one hash check so response time doesn't reveal whether the
    // email has an account.
    let (user, hash) = match row {
        Some((id, email, hash)) => (Some(User { id, email }), hash),
        None => (None, dummy_hash().to_string()),
    };
    let matches = verify_password(password.to_string(), hash).await?;

    match user {
        Some(user) if matches => Ok(user),
        _ => Err(AuthError::InvalidCredentials),
    }
}

pub async fn find_user(pool: &SqlitePool, id: i64) -> Result<Option<User>, AuthError> {
    let row = sqlx::query_as::<_, (i64, String)>("SELECT id, email FROM users WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(id, email)| User { id, email }))
}

// Argon2 is deliberately slow, so it runs off the async worker threads.
async fn hash_password(password: String) -> Result<String, AuthError> {
    tokio::task::spawn_blocking(move || hash_blocking(&password))
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?
}

async fn verify_password(password: String, hash: String) -> Result<bool, AuthError> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&hash).map_err(|e| AuthError::Internal(e.to_string()))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .map_err(|e| AuthError::Internal(e.to_string()))?
}

fn hash_blocking(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AuthError::Internal(e.to_string()))
}

fn dummy_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| hash_blocking("not-a-real-password").expect("hash dummy password"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;

    const PASSWORD: &str = "correct horse battery";

    #[test]
    fn email_validation() {
        assert_eq!(
            normalize_email("  jake@example.com ").unwrap(),
            "jake@example.com"
        );
        for bad in [
            "",
            "jake",
            "@example.com",
            "jake@",
            "jake@example",
            "ja ke@example.com",
            "jake@.com",
        ] {
            assert!(normalize_email(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn password_length_limits() {
        assert!(validate_password(&"a".repeat(MIN_PASSWORD_LEN - 1)).is_err());
        assert!(validate_password(&"a".repeat(MIN_PASSWORD_LEN)).is_ok());
        assert!(validate_password(&"a".repeat(MAX_PASSWORD_LEN)).is_ok());
        assert!(validate_password(&"a".repeat(MAX_PASSWORD_LEN + 1)).is_err());
    }

    #[tokio::test]
    async fn first_account_allowed_then_signups_close() {
        let pool = test_pool().await;
        assert!(signups_open(&pool, false).await.unwrap());

        let user = create_user(&pool, false, "owner@example.com", PASSWORD)
            .await
            .unwrap();
        assert_eq!(user.email, "owner@example.com");

        assert!(!signups_open(&pool, false).await.unwrap());
        let second = create_user(&pool, false, "someone@example.com", PASSWORD).await;
        assert!(matches!(second, Err(AuthError::SignupsClosed)));
    }

    #[tokio::test]
    async fn open_signups_allow_many_accounts_but_not_duplicates() {
        let pool = test_pool().await;
        create_user(&pool, true, "a@example.com", PASSWORD)
            .await
            .unwrap();
        create_user(&pool, true, "b@example.com", PASSWORD)
            .await
            .unwrap();
        let dup = create_user(&pool, true, "A@Example.com", PASSWORD).await;
        assert!(matches!(dup, Err(AuthError::EmailTaken)));
    }

    #[tokio::test]
    async fn password_is_stored_hashed() {
        let pool = test_pool().await;
        create_user(&pool, true, "a@example.com", PASSWORD)
            .await
            .unwrap();
        let stored: String = sqlx::query_scalar("SELECT password_hash FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(stored.starts_with("$argon2"));
        assert!(!stored.contains(PASSWORD));
    }

    #[tokio::test]
    async fn sign_in_checks_password_and_ignores_email_case() {
        let pool = test_pool().await;
        let created = create_user(&pool, true, "a@example.com", PASSWORD)
            .await
            .unwrap();

        let user = verify_credentials(&pool, " A@EXAMPLE.com", PASSWORD)
            .await
            .unwrap();
        assert_eq!(user, created);

        let wrong = verify_credentials(&pool, "a@example.com", "wrong password!!").await;
        assert!(matches!(wrong, Err(AuthError::InvalidCredentials)));

        let unknown = verify_credentials(&pool, "nobody@example.com", PASSWORD).await;
        assert!(matches!(unknown, Err(AuthError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn find_user_by_id() {
        let pool = test_pool().await;
        let created = create_user(&pool, true, "a@example.com", PASSWORD)
            .await
            .unwrap();
        assert_eq!(find_user(&pool, created.id).await.unwrap(), Some(created));
        assert_eq!(find_user(&pool, 9999).await.unwrap(), None);
    }
}
