import sqlite3
import asyncio
import atexit
import os
from typing import Any, Tuple, List
from core.config import DB_FOLDER

class Database:
    def __init__(self, db_name: str):
        """Connects to database

        Args:
            db_name (str): Database file name
        """
        os.makedirs(DB_FOLDER, exist_ok=True)
        self.db_path = os.path.join(DB_FOLDER, db_name)
        self.conn = sqlite3.connect(
            self.db_path,
            check_same_thread=False,
            isolation_level=None
        )
        self._init_pragmas()
        atexit.register(lambda: self.conn.close())

    def _init_pragmas(self):
        """Inits pragmas for performance optimization
        
        PRAGMA journal_mode=WAL:
        Enables Write-Ahead Logging
        
        PRAGMA synchronous=NORMAL:
        Enables synchronous mode
        
        PRAGMA temp_store=MEMORY:
        Uses in-memory temporary tables for fast access
        
        PRAGMA cache_size=-10000:
        Sets cache size
        
        PRAGMA foreign_keys=ON:
        Enables foreign keys
        """
        cur = self.conn.cursor()
        cur.execute("PRAGMA journal_mode=WAL;")
        cur.execute("PRAGMA synchronous=NORMAL;")
        cur.execute("PRAGMA temp_store=MEMORY;")
        cur.execute("PRAGMA cache_size=-10000;")
        cur.execute("PRAGMA foreign_keys=ON;")
        cur.close()

    def _execute_sync(self, query: str, params: Tuple = (), commit: bool = False) -> List[Tuple[Any]]:
        """Executes SQL query

        Args:
            query (str): SQL query
            params (Tuple, optional): Query parameters. Defaults to ().
            commit (bool, optional): Commit changes. Defaults to False.

        Returns:
            List[Tuple[Any]]: Query result
        """
        cur = self.conn.cursor()
        cur.execute(query, params)
        rows = cur.fetchall()
        if commit:
            self.conn.commit()
        cur.close()
        return rows

    def _executemany_sync(self, query: str, data: List[Tuple], commit: bool = True):
        """Executes SQL query

        Args:
            query (str): SQL query
            data (List[Tuple]): Query parameters
            commit (bool, optional): Commit changes. Defaults to False.
        """
        cur = self.conn.cursor()
        cur.executemany(query, data)
        if commit:
            self.conn.commit()
        cur.close()

    async def execute(self, query: str, params: Tuple = (), commit: bool = False) -> List[Tuple[Any]]:
        """Executes SQL query asynchronously

        Args:
            query (str): SQL query
            params (Tuple, optional): Query parameters. Defaults to ().
            commit (bool, optional): Commit changes. Defaults to False.

        Returns:
            List[Tuple[Any]]: Query result
        """
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(None, self._execute_sync, query, params, commit)

    async def execute_many(self, query: str, data: List[Tuple], commit: bool = True):
        """Executes SQL query asynchronously

        Args:
            query (str): SQL query
            data (List[Tuple]): Query parameters
            commit (bool, optional): Commit changes. Defaults to False.
        """
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(None, self._executemany_sync, query, data, commit)
