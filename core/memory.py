from collections import ChainMap
import asyncio
import json

class MemoryStore:
    def __init__(self):
        self.lock = asyncio.Lock()
        self.maps = ChainMap(
            {"users": {}},
            {"sessions": {}},
            {"stats": {}}
        )

    async def get(self, category, key):
        async with self.lock:
            return self.maps[category].get(key)

    async def set(self, category, key, value):
        async with self.lock:
            self.maps[category][key] = value

    async def save_snapshot(self, path="memory_backup.json"):
        async with self.lock:
            with open(path, "w") as f:
                json.dump(self.maps.maps[0], f)

    async def load_snapshot(self, path="memory_backup.json"):
        async with self.lock:
            # solo el primer mapa es editable
            with open(path) as f:
                self.maps.maps[0].update(json.load(f))
