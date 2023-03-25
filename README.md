# aioworldline
Unofficial Worldline portal data retrieving client

Environment variables to set:
```
WORLDLINE_ACCOUNT_ID=
WORLDLINE_LOGIN=
WORLDLINE_PASSWORD=
```

Example usage:
```python
from datetime import date
from aioworldline import worldline

date_from = date(2023, 1, 1)
date_till = date(2023, 1, 1)

async with worldline.login() as session:
    csv_data = await worldline.get_transaction_report(session, date_from, date_till)
```
