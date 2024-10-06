import asyncio
import csv
import logging
from datetime import timedelta, date
from io import StringIO

from aioworldline import worldline

logger = logging.getLogger(__name__)


async def main():
    date_from = date.today() - timedelta(days=5)
    current_date_till = date_from

    async with worldline.login(timeout=15 * 60) as wl_session:
        try:
            csv_data = await worldline.get_transaction_report(wl_session, date_from, current_date_till)
        except Exception as e:
            logger.error(f'Failed reading the response from Worldline: {e}')

            raise

        reader = csv.DictReader(StringIO(csv_data.decode('utf-8-sig')), dialect='unix', delimiter=';')

        for row in reader:
            logger.info(row)


asyncio.run(main())
