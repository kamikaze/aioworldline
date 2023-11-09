from pydantic import SecretStr
from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    login: str | None
    password: SecretStr | None
    account_id: str | None

    class Config(BaseSettings.Config):
        env_prefix = 'WORLDLINE_'


settings = Settings()
