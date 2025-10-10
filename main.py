import asyncio
from cli.panel import cli_panel
from core.logger import log
from core.db_instances import instance_databases
from network.tcp_server import start_server
import sys
import traceback

if sys.platform != "win32":
    try:
        import uvloop
        asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
        print("✅ uvloop enabled")
    except ImportError:
        print("⚠️ uvloop not installed")
else:
    print("⚠️ Windows detected, using normal asyncio")

async def restart_on_crash(coro_func, *args, delay=1, name=None):
    """
    Executes a coroutine and if it crashes it restarts
    """
    while True:
        try:
            await coro_func(*args)
            break
        except asyncio.CancelledError:
            break
        except Exception:
            log.error(f"Task {name or coro_func.__name__} crashed, restarting...")
            traceback.print_exc()
            await asyncio.sleep(delay)

async def main():
    """
        Master Server Entrypoint
        
        Starts all the tasks and tries to not fuck it up
    """
    db_task = asyncio.create_task(restart_on_crash(instance_databases, name="DB Init"))
    tcp_task = asyncio.create_task(restart_on_crash(start_server, name="TCP Server"))
    cli_task = asyncio.create_task(cli_panel())

    tasks = [db_task, tcp_task, cli_task]

    await cli_task

    for task in tasks:
        task.cancel()
        try:
            await task
        except asyncio.CancelledError:
            pass

    log.info("Server stopped cleanly")

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        log.info("Server stopped by user")
