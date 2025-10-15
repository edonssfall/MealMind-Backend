use argon2::{password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Argon2};
use rand_core::OsRng;
use tracing::error;

pub fn hash_password(plain: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| {
            error!(error = %e, "argon2 hash_password error");
            anyhow::anyhow!(e.to_string())
        })?
        .to_string();
    Ok(hash)
}

pub fn verify_password(plain: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| {
        error!(error = %e, "argon2 parse hash error");
        anyhow::anyhow!(e.to_string())
    })?;
    Ok(Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok())
}
