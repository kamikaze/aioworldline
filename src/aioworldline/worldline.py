import logging
from asyncio import sleep
from contextlib import asynccontextmanager
from datetime import date
from html.parser import HTMLParser

from aiohttp import ClientSession, ClientTimeout, ClientOSError
from pydantic import SecretStr

from aioworldline.conf import settings

logger = logging.getLogger(__name__)
BASE_URL = 'https://portal.baltic.worldline-solutions.com'
LOGIN_PAGE_URL = f'{BASE_URL}/fdmp/login.jsp'
AUTH_URL = f'{BASE_URL}/fdmp/j_security_check'
MERCHANT_SWITCH_URL = f'{BASE_URL}/fdmp/transaction_info'
DETAILED_TURNOVER_PAGE_URL = f'{BASE_URL}/fdmp/detailed_turnover'
EXPORT_LIST_DATA_URL = f'{BASE_URL}/fdmp/export_list_data'


class WLHTMLParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.csrf_value: str | None = None

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        value = None
        is_csrf = False

        for attr in attrs:
            if (tag_name := attr[0].lower()) == 'id':
                if attr[1].upper() != '__CSRF':
                    break

                is_csrf = True
            elif tag_name == 'value':
                value = attr[1]

        if is_csrf:
            self.csrf_value = value


def _get_csrf_value(html_page: str) -> str | None:
    parser = WLHTMLParser()
    parser.feed(html_page)

    if parser.csrf_value is None:
        msg = 'Unable to extract CSRF value from the page'
        logger.error(msg)

        raise ValueError(msg)

    return parser.csrf_value


@asynccontextmanager
async def login(username: str = settings.login, password: SecretStr = settings.password, timeout: int = None):
    params = {
        '__Action': 'login:b_login#Save#',
        'j_username': username,
        'j_password': password.get_secret_value(),
    }
    headers = {
        'User-Agent': 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/111.0'
    }

    async with ClientSession(headers=headers, timeout=ClientTimeout(total=timeout)) as session:
        logger.debug('Opening login page')

        async with session.get(LOGIN_PAGE_URL):
            pass

        await sleep(5)

        logger.debug('Authenticating')

        async with session.post(AUTH_URL, data=params):
            pass

        await sleep(5)

        yield session


async def get_transaction_report(session: ClientSession, date_from: date, date_till: date,
                                 account_id: str = settings.account_id, date_type: str = 'D', use_date: str = 'TR',
                                 merchant: str = None, term_id: str = None, report_type: str = 'detailed_turnover',
                                 export_type: str = 'csv') -> bytes:
    params = {
        '__Action': 'merchant:parent_id',
        '__CSRF': 'null',
        'merchant:parent_id': str(account_id),
        'transaction_info:news_id': '',
    }

    logger.debug(f'Switching merchant account to {account_id}')

    async with session.post(MERCHANT_SWITCH_URL, params=params):
        pass

    await sleep(10)

    params = {
        'group': 'tab.detailed_turnover',
    }

    logger.debug(f'Opening detailed turnover report page')

    async with session.get(DETAILED_TURNOVER_PAGE_URL, params=params):
        pass

    await sleep(10)

    logger.debug(f'Exporting transactions for {date_from} - {date_till}')

    params = {
        'uniqueid': 'detailed_turnover:detailed_turnover_search_result',
        'exportType': export_type,
        'page': '1',
        'countRow': '15',
        'sortField': '',
        'sortType': '0',
        'detailed_turnover:date_type': date_type,
        'detailed_turnover:parent': account_id,
        'detailed_turnover:shipm_date_from': date_from.strftime('%d.%m.%Y'),
        'detailed_turnover:shipm_date_till': date_till.strftime('%d.%m.%Y'),
        'detailed_turnover:use_date': use_date,
        'detailed_turnover:merchant_ret': 'detailed_turnover:merchant~{merchant}|detailed_turnover:merchant_txt'
                                          '~{merchant} {full_name}|detailed_turnover:merchant_order~{merchant}',
        'detailed_turnover:term_id_ret': 'detailed_turnover:term_id~{terminal_id}|detailed_turnover:term_id_txt'
                                         '~{terminal_id} {term_type}|detailed_turnover:term_id_order~{terminal_id}',
    }

    async with session.get(EXPORT_LIST_DATA_URL, params=params, allow_redirects=True) as response:
        try:
            return await response.read()
        except ClientOSError as e:
            logger.error(f'Failed reading the response from Worldline: {e}')

            raise
        except Exception as e:
            logger.error(f'Failed reading the response from Worldline: {e}')

            raise
