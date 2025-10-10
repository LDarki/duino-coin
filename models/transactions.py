from core.db import Database
from time import time

class TransactionModel:
    def __init__(self, db: Database):
        self.db = db

    @classmethod
    async def create(cls, db: Database):
        self = cls(db)
        await self.create_table()
        return self

    async def create_table(self):
        await self.db.execute(
            """
            CREATE TABLE
            IF NOT EXISTS
            Transactions(
            id INTEGER
            NOT NULL PRIMARY KEY
            AUTOINCREMENT UNIQUE,
            timestamp TEXT,
            username TEXT,
            recipient TEXT,
            amount REAL,
            hash TEXT,
            memo TEXT)
            """,
            commit=True
        )

    async def get_transaction(self, id: int):
        res = await self.db.execute(
            "SELECT * FROM Transactions WHERE id=?", (id,)
        )
        return res[0] if res else None
    
    async def add_transaction(self, username: str, recipient: str, amount: float, hash: str, memo: str):
        await self.db.execute(
            "INSERT INTO Transactions (timestamp, username, recipient, amount, hash, memo) VALUES (?,?,?,?,?,?)",
            (int(time()), username, recipient, amount, hash, memo),
            commit=True
        )

    async def delete_transaction(self, id: id):
        await self.db.execute(
            "DELETE FROM Transactions WHERE id=?",
            (id,),
            commit=True
        )