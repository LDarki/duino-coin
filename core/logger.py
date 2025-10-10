import logging
from colorama import Fore, Style, init

init(autoreset=True)

def setup_logger(name="DUCO"):
    """
    Sets up logger
    
    Args:
        name (str): Logger name
    
    Returns:
        Logger
    """
    logger = logging.getLogger(name)
    logger.setLevel(logging.INFO)
    ch = logging.StreamHandler()
    ch.setLevel(logging.INFO)
    formatter = logging.Formatter(
        f"{Fore.YELLOW}[%(asctime)s]{Style.RESET_ALL} %(message)s", "%H:%M:%S"
    )
    ch.setFormatter(formatter)
    logger.addHandler(ch)
    return logger

log = setup_logger()
