import asyncio
from core.logger import log
from core.config import SERVER_VERSION, SOCKET_TIMEOUT, TCP_PORT, TCP_HOST

async def handle_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    from core.db_instances import users_model
    """
    Handler for each connected client.
    """
    addr = writer.get_extra_info('peername')
    log.info(f"Client connected: {addr}")
    
    await send_data(SERVER_VERSION, writer)
    
    try:
        while True:
            
            try:
                data = await asyncio.wait_for(receive_data(reader), timeout=SOCKET_TIMEOUT)
            except asyncio.TimeoutError:
                log.info(f"Timeout: disconnecting {addr}")
                break
            
            if not data:
                break

            cmd = data[0].lower()
            
            # -----------------
            # USER COMMANDS
            # -----------------
            if cmd == "bala":
                
                username = data[1]
                
                user = await users_model.get_user(username)
                if user:
                    await send_data(f"{user[3]}", writer)
                else:
                    await send_data(f"NO,User {username} not found", writer)
            else:
                await send_data(f"NO,Unknown command: {cmd}", writer)

    except asyncio.CancelledError:
        log.info(f"Client task cancelled: {addr}")
    except Exception as e:
        log.error(f"Client {addr} exception: {e}")
    finally:
        writer.close()
        await writer.wait_closed()
        log.info(f"Client disconnected: {addr}")


async def receive_data(reader: asyncio.StreamReader, limit=2048):
    """
    Receives data from the client asynchronously.
    
    Args:
        reader (asyncio.StreamReader): StreamReader for the client
        limit (int, optional): Max bytes to read at once
    
    Returns:
        list[str]: Split list of commands, or None if client disconnected
    """
    try:
        data_bytes = await reader.read(limit)
        if not data_bytes:
            return None
        data_str = data_bytes.decode().strip()
        if not data_str:
            return None 
        return data_str.split(",")
    except Exception:
        return None


async def send_data(message: str, writer: asyncio.StreamWriter):
    """
    Sends a message to the client asynchronously.
    
    Args:
        message (str): Message to send
        writer (asyncio.StreamWriter): StreamWriter for the client
    """
    if not isinstance(message, str):
        message = str(message)
    writer.write((message + "\n").encode())
    await writer.drain()


async def start_server():
    """
    Starts the asynchronous TCP server.
    Accepts multiple clients concurrently.
    """
    server = await asyncio.start_server(handle_client, TCP_HOST, TCP_PORT)
    log.info(f"TCP server started on {TCP_HOST}:{TCP_PORT}\n")
    async with server:
        await server.serve_forever()