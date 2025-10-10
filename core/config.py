import os

SERVER_VERSION = 4.0

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DB_FOLDER = f"{BASE_DIR}/db"

DATABASES = {
    "users": f"{DB_FOLDER}/crypto_database.db",
    "transactions": f"{DB_FOLDER}/transactions.db",
    "stats": f"{DB_FOLDER}/stats.db"
}

BCRYPT_ROUNDS=6

TCP_HOST = "127.0.0.1"
TCP_PORT = 4000
SOCKET_TIMEOUT = 60