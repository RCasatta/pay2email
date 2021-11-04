#!/usr/bin/env python3
from lightning import Plugin
import requests
import os

plugin = Plugin()


@plugin.init()
def init(options, configuration, plugin):
    plugin.log("Plugin on_pay initialize")


@plugin.subscribe("invoice_payment")
def on_invoice_payment(plugin, invoice_payment):
    plugin.log("on_pay on_invoice_payment")
    url = plugin.options['on-pay-notify-url']['value']
    s = requests.Session()
    s.auth = (os.getenv("HTTP_AUTH_USER"), os.getenv("HTTP_AUTH_PASSWORD"))
    s.post(url, data=invoice_payment.preimage)
    # TODO, if not 200 save somewhere and retry later


plugin.add_option('on-pay-notify-url', 'https://pay2.email/invoice/paid', 'The url where to notify the paid invoice')

plugin.run()