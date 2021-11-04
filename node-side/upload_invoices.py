#!/usr/bin/env python3
from lightning import LightningRpc
import requests
import uuid
import random
import os

s = requests.Session()
s.auth = (os.getenv("HTTP_AUTH_USER"), os.getenv("HTTP_AUTH_PASSWORD"))
r = s.get('https://pay2.email/invoice/count')
available = int(r.text)
print(available)
l1 = LightningRpc(os.getenv("LIGHTNING_RPC"))
for i in range(available, 60):
    expire = "{}d".format(random.randint(7, 28))  # random so that invoices don't expire at the same time
    invoice = l1.invoice("20sat", str(uuid.uuid4()), "pay2.email", expire)['bolt11']
    s.post('https://pay2.email/invoice', invoice)
