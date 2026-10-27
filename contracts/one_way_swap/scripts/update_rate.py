#!/usr/bin/env python3
"""Poll a price API and set the one-way-swap rate. One run sends at most one update; schedule it with cron.

The API must report the USDC price of one whole out_denom token (e.g. VERONA in USD).
The contract rate is out_denom base units per 1 in_denom base unit:

    rate = 10^(OUT_DECIMALS - IN_DECIMALS) / price      (floored to 18 decimal places)

Environment:
  CONTRACT      swap contract address                                    (required)
  KEY           xiond keyring key that signs                             (required)
  PRICE_URL     JSON endpoint                                            (required)
  PRICE_FIELD   dot path to the price in the response, e.g. verona.usd   (required)
  GRANTER       contract admin address. When set, KEY sends UpdateRate through authz on
                behalf of GRANTER and never holds admin power. Unset: KEY must be the admin.
  IN_DECIMALS=6  OUT_DECIMALS=6
  MIN_CHANGE=0.005    skip moves smaller than this fraction of the current rate
  MAX_CHANGE=0.10     refuse moves larger than this fraction; apply those by hand
  RATE_MIN, RATE_MAX  optional absolute bounds; refuse anything outside them
  NODE=https://rpc.xion-testnet-2.burnt.com:443  CHAIN_ID=xion-testnet-2
  GAS_PRICES=0.025uxion  KEYRING_BACKEND (xiond's default if unset)
  DRY_RUN=1           decide and log, send nothing

Exit status: 0 updated or nothing to do, 1 error, 2 refused by a guard (alert on this).

The admin key can also Withdraw every balance and hand off admin, so a bot should not hold it.
Give the bot its own key and grant it update_rate only, from the admin:

  xiond tx wasm grant contract <bot-address> execution <contract> \\
      --allow-msg-keys update_rate --no-token-transfer --from <admin-key> ...

then run with KEY=<bot-key> GRANTER=<admin-address>.

Cron, every 10 minutes:
  */10 * * * * set -a; . /etc/one-way-swap/rate.env; /opt/one-way-swap/update_rate.py >> /var/log/one-way-swap-rate.log 2>&1

Self-test: python3 -m doctest update_rate.py
"""

import json
import os
import subprocess
import sys
import tempfile
import urllib.request
from datetime import datetime, timezone
from decimal import ROUND_DOWN, Decimal, getcontext

getcontext().prec = 60  # cosmwasm Decimal values reach 39 significant digits
STEP = Decimal("1e-18")  # cosmwasm Decimal resolution


def rate_from_price(price, in_decimals, out_decimals):
    """Contract rate string for a USDC price of one whole out_denom token.

    >>> rate_from_price(Decimal("0.5"), 6, 6)
    '2'
    >>> rate_from_price(Decimal("2"), 6, 18)
    '500000000000'
    >>> rate_from_price(Decimal("3"), 6, 6)
    '0.333333333333333333'
    >>> rate_from_price(Decimal("0"), 6, 6)
    Traceback (most recent call last):
    ...
    ValueError: price must be positive, got 0
    """
    if not price.is_finite() or price <= 0:
        raise ValueError(f"price must be positive, got {price}")
    rate = (Decimal(10) ** (out_decimals - in_decimals) / price).quantize(STEP, rounding=ROUND_DOWN)
    if rate <= 0:
        raise ValueError(f"price {price} floors the rate to zero")
    return format(rate.normalize(), "f")


def decide(current, new, min_change, max_change, rate_min=None, rate_max=None):
    """Return (action, reason) where action is "update", "skip" or "refuse".

    >>> decide(Decimal(100), Decimal("100.4"), Decimal("0.005"), Decimal("0.1"))[0]
    'skip'
    >>> decide(Decimal(100), Decimal(105), Decimal("0.005"), Decimal("0.1"))[0]
    'update'
    >>> decide(Decimal(100), Decimal(111), Decimal("0.005"), Decimal("0.1"))
    ('refuse', 'change 11% exceeds MAX_CHANGE 10%')
    >>> decide(Decimal(100), Decimal(105), Decimal("0.005"), Decimal("0.1"), rate_max=Decimal(104))
    ('refuse', 'rate 105 outside [None, 104]')
    """
    change = abs(new - current) / current
    pct = f"{float(change) * 100:.3g}%"
    if (rate_min is not None and new < rate_min) or (rate_max is not None and new > rate_max):
        return "refuse", f"rate {new} outside [{rate_min}, {rate_max}]"
    if change > max_change:
        return "refuse", f"change {pct} exceeds MAX_CHANGE {float(max_change) * 100:.3g}%"
    if change < min_change:
        return "skip", f"change {pct} below MIN_CHANGE"
    return "update", f"change {pct}"


def fetch_price(url, field):
    req = urllib.request.Request(url, headers={"Accept": "application/json", "User-Agent": "one-way-swap-rate"})
    with urllib.request.urlopen(req, timeout=15) as resp:
        value = json.load(resp, parse_float=Decimal)
    for part in field.split("."):
        value = value[int(part)] if isinstance(value, list) else value[part]
    return Decimal(str(value))


def xiond(*args, node):
    res = subprocess.run(["xiond", *args, "--node", node, "--output", "json"], capture_output=True, text=True)
    if res.returncode != 0:
        raise RuntimeError(f"xiond {' '.join(args[:3])}: {(res.stderr or res.stdout).strip()}")
    return json.loads(res.stdout)


def send_update(env, contract, rate, node, chain_id):
    msg = json.dumps({"update_rate": {"rate": rate}})
    signing = ["--from", env["KEY"], "--chain-id", chain_id, "--gas", "auto", "--gas-adjustment", "1.3",
               "--gas-prices", env.get("GAS_PRICES", "0.025uxion"), "-y"]
    if "KEYRING_BACKEND" in env:
        signing += ["--keyring-backend", env["KEYRING_BACKEND"]]
    if env.get("GRANTER"):
        unsigned = xiond("tx", "wasm", "execute", contract, msg, "--from", env["GRANTER"],
                         "--chain-id", chain_id, "--generate-only", node=node)
        with tempfile.NamedTemporaryFile("w", suffix=".json") as f:
            json.dump(unsigned, f)
            f.flush()
            res = xiond("tx", "authz", "exec", f.name, *signing, node=node)
    else:
        res = xiond("tx", "wasm", "execute", contract, msg, *signing, node=node)
    if res.get("code"):
        raise RuntimeError(f"tx rejected: {res.get('raw_log')}")
    included = xiond("query", "wait-tx", res["txhash"], "--timeout", "60s", node=node)
    if included.get("code"):
        raise RuntimeError(f"tx {res['txhash']} failed: {included.get('raw_log')}")
    return res["txhash"]


def log(msg):
    print(f"{datetime.now(timezone.utc).isoformat(timespec='seconds')} {msg}", flush=True)


def main():
    env = os.environ
    missing = [k for k in ("CONTRACT", "KEY", "PRICE_URL", "PRICE_FIELD") if not env.get(k)]
    if missing:
        sys.exit(f"missing required env: {', '.join(missing)}")
    contract = env["CONTRACT"]
    node = env.get("NODE", "https://rpc.xion-testnet-2.burnt.com:443")
    chain_id = env.get("CHAIN_ID", "xion-testnet-2")

    def bound(name):
        return Decimal(env[name]) if env.get(name) else None

    price = fetch_price(env["PRICE_URL"], env["PRICE_FIELD"])
    new = rate_from_price(price, int(env.get("IN_DECIMALS", 6)), int(env.get("OUT_DECIMALS", 6)))
    current = xiond("query", "wasm", "contract-state", "smart", contract, '{"config":{}}', node=node)["data"]["rate"]
    action, reason = decide(Decimal(current), Decimal(new), Decimal(env.get("MIN_CHANGE", "0.005")),
                            Decimal(env.get("MAX_CHANGE", "0.10")), bound("RATE_MIN"), bound("RATE_MAX"))
    log(f"price={price} current={current} new={new} -> {action}: {reason}")
    if action == "refuse":
        return 2
    if action == "skip" or env.get("DRY_RUN"):
        return 0
    log(f"rate set to {new} in tx {send_update(env, contract, new, node, chain_id)}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # one log line per failed run instead of a traceback in the cron log
        log(f"error: {e}")
        sys.exit(1)
