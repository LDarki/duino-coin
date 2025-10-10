import functools
import traceback

def catch_errors(func):
    """Decorator to catch errors in async functions"""
    @functools.wraps(func)
    async def wrapper(*args, **kwargs):
        try:
            return await func(*args, **kwargs) or True
        except Exception as e:
            print(f"[ERROR] {func.__name__} failed: {e}")
            traceback.print_exc()
            return False
    return wrapper
