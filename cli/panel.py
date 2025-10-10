import asyncio
from datetime import datetime
from colorama import Fore, Style, init
from core.logger import log
from core.config import BCRYPT_ROUNDS
import bcrypt
import os
import sys
import psutil

init(autoreset=True)

COLOR_MAP = {
    "error": Fore.RED,
    "success": Fore.GREEN,
    "warning": Fore.YELLOW,
    "?": Fore.CYAN
}

def beautify_print(message: str):
    fg_color = next((color for key, color in COLOR_MAP.items() if key.lower() in message.lower()), Fore.WHITE)
    timestamp = datetime.now().strftime(Style.DIM + Fore.WHITE + "%H:%M:%S.%f:")
    print(f"{timestamp} {Style.BRIGHT}{fg_color}{message}")

async def cli_panel():
    while True:
        prompt = (
            datetime.now().strftime(Style.DIM + Fore.WHITE + "%H:%M:%S.%f:")
            + Style.BRIGHT
            + Fore.YELLOW
            + " DUCO Server $ "
            + Style.RESET_ALL
        )

        cmd = await asyncio.to_thread(input, prompt)
        cmd = cmd.strip().split()

        if not cmd:
            continue

        if cmd[0] == "exit":
            log.info("Exiting CLI...")
            break

        elif cmd[0] == "help":
            beautify_print("""
                Available commands:
                - exit - exits DUCO server
                - restart - restarts DUCO server
                - help - shows this help menu
                - user add <username> <password> <email> <balance>
                - user info <username>
                - user del <username>
            """)
            
        elif cmd[0] == "restart":
            beautify_print("Are you sure you want to restart DUCO server?")
            confirm = await asyncio.to_thread(input, " Y/n")
            if confirm.lower() in ("y", ""):
                os.system("sudo pkill -9 rsync")
                
                current_pid = os.getpid()
                current_proc = psutil.Process(current_pid)

                for child in current_proc.children(recursive=True):
                    try:
                        child.kill()
                    except Exception as e:
                        beautify_print(f"Error killing child process {child.pid}: {e}")
                
                os.execl(sys.executable, sys.executable, *sys.argv)
            else:
                beautify_print("Canceled")

        elif cmd[0] == "user":
            from core.db_instances import users_model
            
            if len(cmd) < 2:
                beautify_print("Error: Missing user subcommand (add/info/del)")
                continue

            subcmd = cmd[1]

            if subcmd == "add" and len(cmd) == 6:
                username, password, email, balance = cmd[2], cmd[3], cmd[4], cmd[5]
                hashed_pass = bcrypt.hashpw(password.encode("utf-8"), bcrypt.gensalt(rounds=BCRYPT_ROUNDS))
                await users_model.add_user(username, hashed_pass, email, balance)
                beautify_print(f"Success: User {username} added.")
            
            elif subcmd == "info" and len(cmd) == 3:
                user = await users_model.get_user(cmd[2])
                beautify_print(f"Info: {user}")
            
            elif subcmd == "del" and len(cmd) == 3:
                await users_model.delete_user(cmd[2])
                beautify_print(f"Success: User {cmd[2]} deleted.")
            
            else:
                beautify_print("""
                    Usage:
                    - user add <username> <password> <email> <balance>
                    - user info <username>
                    - user del <username>
                """)

        else:
            beautify_print(f"Unknown command: {' '.join(cmd)}")
