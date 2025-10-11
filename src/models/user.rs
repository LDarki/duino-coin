use sqlx::{Pool, Sqlite, Row, Column};
use anyhow::Result;

#[derive(Clone)]
pub struct UserModel {
    pub pool: Pool<Sqlite>,
}

#[derive(Debug)]
pub struct User {
    pub username: String,
    pub password: String,
    pub email: String,
    pub balance: f64,
    pub created: Option<String>,
    pub rig_verified: Option<String>,
    pub stake: Option<i64>,
}

impl UserModel {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn get_user(&self, username: &str) -> Result<Option<User>> {
        if let Some(row) = sqlx::query("SELECT username, password, email, balance, created, rig_verified, stake FROM Users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?
        {
            let password_blob: Vec<u8> = row.try_get("password")?;
            let password = String::from_utf8(password_blob)
                .unwrap_or_else(|_| "<invalid utf8>".to_string());

            let user = User {
                username: row.try_get("username")?,
                password,
                email: row.try_get("email")?,
                balance: row.try_get("balance")?,
                created: row.try_get("created")?,
                rig_verified: row.try_get("rig_verified")?,
                stake: row.try_get("stake")?,
            };
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    pub async fn add_user(&self, username: &str, password_hash: &str, email: &str, balance: f64) -> Result<()> {
        sqlx::query("INSERT INTO Users (username, password, email, balance) VALUES (?, ?, ?, ?)")
            .bind(username)
            .bind(password_hash)
            .bind(email)
            .bind(balance)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_user(&self, username: &str) -> Result<()> {
        sqlx::query("DELETE FROM Users WHERE username = ?")
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_user(&self, username: &str, email: &str, balance: &str) -> Result<()> {
        sqlx::query("UPDATE Users SET email = ?, balance = ? WHERE username = ?")
            .bind(email)
            .bind(balance)
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_password(&self, username: &str, new_password_hash: &str) -> Result<()> {
        sqlx::query("UPDATE Users SET password = ? WHERE username = ?")
            .bind(new_password_hash)
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
