from core.db import Database
from models.user import UserModel
from core.config import DATABASES
from core.logger import log
from datetime import datetime

users_db: Database = None
users_model: UserModel = None

async def instance_databases():
    """Instances databases
    """
    global users_db, users_model

    start_time = datetime.now()

    users_db = Database(DATABASES['users'])
    users_model = await UserModel.create(users_db)
    
    users_count = len(await users_model.get_all_users())

    elapsed = (datetime.now() - start_time).total_seconds()
    log.info(f"Users database initialized with {users_count} users in {elapsed:.3f} seconds")
