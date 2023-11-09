from pydantic import SecretStr
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    login: str | None
    password: SecretStr | None
    account_id: str | None

    model_config = SettingsConfigDict(env_prefix='WORLDLINE_')


settings = Settings()
