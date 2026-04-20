from datetime import date

class WorldlineSession:
    async def get_transaction_report(
        self,
        date_from: date,
        date_till: date,
        account_id: str,
    ) -> bytes: ...

class Login:
    async def __aenter__(self) -> WorldlineSession: ...
    async def __aexit__(
        self,
        exc_type: object,
        exc_val: object,
        exc_tb: object,
    ) -> bool: ...

def login(
    username: str,
    password: str,
    timeout: int | None = None,
) -> Login: ...
