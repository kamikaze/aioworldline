from pydantic import SecretStr


class WorldlineSettingsMixin:
    worldline_login: str | None
    worldline_password: SecretStr | None
    worldline_account_id: str | None
