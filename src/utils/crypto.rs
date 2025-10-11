use anyhow::{anyhow, Result};
use argon2::{
    Argon2,
    PasswordHasher,
    PasswordVerifier,
    password_hash::{SaltString, PasswordHash}
};
use bcrypt::verify as bcrypt_verify;
use rand::rngs::OsRng;

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!(e.to_string()))?
        .to_string();

    Ok(hash)
}

pub fn verify_and_upgrade(password: &str, stored_hash: &str) -> Result<Option<String>> {
    if stored_hash.starts_with("$2b$") || stored_hash.starts_with("$2a$") {
        if bcrypt_verify(password, stored_hash).map_err(|e| anyhow!(e.to_string()))? {
            let new_hash = hash_password(password)?;
            Ok(Some(new_hash))
        } else {
            Ok(None)
        }
    }
    else if stored_hash.starts_with("$argon2") {
        let parsed =
            PasswordHash::new(stored_hash).map_err(|e| anyhow!(e.to_string()))?;
        let argon2 = Argon2::default();

        if argon2
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
        {
            Ok(Some(stored_hash.to_string()))
        } else {
            Ok(None)
        }
    }
    else {
        Ok(None)
    }
}
