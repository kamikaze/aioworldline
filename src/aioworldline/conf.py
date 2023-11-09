from pydantic import SecretStr
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    login: str | None = None
    password: SecretStr | None = None
    account_id: str | None = None

    model_config = SettingsConfigDict(env_prefix='WORLDLINE_')


settings = Settings()
