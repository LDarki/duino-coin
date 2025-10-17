use sqlx::{Pool, Sqlite, Row};
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct TransactionModel {
    pub pool: Pool<Sqlite>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub timestamp: String,
    pub username: String,
    pub recipient: String,
    pub amount: f64,
    pub hash: String,
    pub memo: String,
    pub id: i64,
}

impl TransactionModel {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn get_all(&self) -> Result<Vec<Transaction>> {
        let rows = sqlx::query("SELECT timestamp, username, recipient, amount, hash, memo, id FROM Transactions")
            .fetch_all(&self.pool)
            .await?;

        let mut transactions = Vec::new();
        for row in rows {
            transactions.push(Transaction {
                timestamp: row.try_get("timestamp")?,
                username: row.try_get("username")?,
                recipient: row.try_get("recipient")?,
                amount: row.try_get("amount")?,
                hash: row.try_get("hash")?,
                memo: row.try_get("memo")?,
                id: row.try_get("id")?,
            });
        }

        Ok(transactions)
    }

    pub async fn get_by_username(&self, username: &str) -> Result<Vec<Transaction>> {
        let rows = sqlx::query("SELECT timestamp, username, recipient, amount, hash, memo, id FROM Transactions WHERE username = ? OR recipient = ?")
            .bind(username)
            .fetch_all(&self.pool)
            .await?;

        let mut transactions = Vec::new();
        for row in rows {
            transactions.push(Transaction {
                timestamp: row.try_get("timestamp")?,
                username: row.try_get("username")?,
                recipient: row.try_get("recipient")?,
                amount: row.try_get("amount")?,
                hash: row.try_get("hash")?,
                memo: row.try_get("memo")?,
                id: row.try_get("id")?,
            });
        }

        Ok(transactions)
    }

    pub async fn add_transaction(
        &self,
        timestamp: &str,
        username: &str,
        recipient: &str,
        amount: f64,
        hash: &str,
        memo: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO Transactions (timestamp, username, recipient, amount, hash, memo) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(timestamp)
        .bind(username)
        .bind(recipient)
        .bind(amount)
        .bind(hash)
        .bind(memo)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
