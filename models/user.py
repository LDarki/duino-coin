from core.db import Database
from time import time

class UserModel:
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
            CREATE TABLE IF NOT EXISTS Users(
                username TEXT PRIMARY KEY,
                password TEXT,
                email TEXT,
                balance REAL DEFAULT 0,
                created INTEGER DEFAULT 0,
                rig_verified TEXT DEFAULT 'No',
                last_seen INTEGER DEFAULT 0,
                stake INTEGER DEFAULT 0
            )
            """,
            commit=True
        )
        
    async def get_all_users(self):
        res = await self.db.execute("SELECT * FROM Users")
        return res

    async def get_user(self, username: str):
        res = await self.db.execute(
            "SELECT * FROM Users WHERE username=?", (username,)
        )
        return res[0] if res else None

    async def add_user(self, username: str, password: str, email: str, balance: float = 0):
        await self.db.execute(
            "INSERT INTO Users (username,password,email,balance,created) VALUES (?,?,?,?,?)",
            (username, password, email, balance, int(time())),
            commit=True
        )

    async def update_balance(self, username: str, new_balance: float):
        await self.db.execute(
            "UPDATE Users SET balance=? WHERE username=?",
            (new_balance, username),
            commit=True
        )

    async def delete_user(self, username: str):
        await self.db.execute(
            "DELETE FROM Users WHERE username=?",
            (username,),
            commit=True
        )