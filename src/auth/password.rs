use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let password = "Secur3P@ssw0rd!";
        let hash = hash_password(password).expect("hashing should succeed");
        assert!(verify_password(password, &hash).expect("verify should succeed"));
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let password = "correct-horse-battery-staple";
        let hash = hash_password(password).expect("hashing should succeed");
        assert!(!verify_password("wrong-password", &hash).expect("verify should not error"));
    }

    #[test]
    fn verify_errors_on_malformed_hash() {
        let err = verify_password("anything", "not-a-valid-hash").unwrap_err();
        let msg = err.to_string();
        assert!(msg.len() > 0);
    }
}
