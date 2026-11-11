"""Offline: signs {licensee, expires_at} with the private key and writes a licence JSON file.

Examples, on your laptop:
    uv run --project judge python scripts/license_sign.py --licensee "Demo Bank" --expires-in 5m \
        --out licence-5min.json
    uv run --project judge python scripts/license_sign.py --licensee "Demo Bank" --expires-in 365d \
        --out licence-long.json

Owner: B. Filled in by: B3, R5 (no longer imports the Python API).
"""

from __future__ import annotations

import argparse
import base64
import json
import re
import sys
from datetime import UTC, datetime, timedelta, timezone
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

REPO = Path(__file__).resolve().parent.parent
DEFAULT_KEY = Path.home() / ".scam-map" / "license.key"
IST = timezone(timedelta(hours=5, minutes=30))
UNITS = {"s": "seconds", "m": "minutes", "h": "hours", "d": "days"}


def parse_duration(text: str) -> timedelta:
    match = re.fullmatch(r"(\d+)([smhd])", text.strip())
    if match is None:
        raise argparse.ArgumentTypeError("use a number and a unit, like 30s, 5m, 2h or 365d")
    return timedelta(**{UNITS[match.group(2)]: int(match.group(1))})


def canonical_bytes(licensee: str, expires_at: str) -> bytes:
    """The exact bytes that are signed; api-rs/src/license/verify.rs builds the same ones."""
    doc = {"licensee": licensee, "expires_at": expires_at}
    return json.dumps(doc, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Sign a licence file.")
    parser.add_argument("--licensee", required=True)
    when = parser.add_mutually_exclusive_group(required=True)
    when.add_argument("--expires-in", type=parse_duration, help="for example 5m or 365d")
    when.add_argument("--expires-at", help="ISO 8601 time with an offset")
    parser.add_argument("--key", type=Path, default=DEFAULT_KEY, help="private key path")
    parser.add_argument("--out", type=Path, help="write here instead of printing")
    args = parser.parse_args(argv)

    if args.expires_at:
        expires = datetime.fromisoformat(args.expires_at)
        if expires.tzinfo is None:
            parser.error("--expires-at needs a timezone offset, like +05:30")
    else:
        expires = datetime.now(UTC) + args.expires_in
    expires_at = expires.astimezone(IST).replace(microsecond=0).isoformat()

    key = serialization.load_pem_private_key(args.key.expanduser().read_bytes(), password=None)
    if not isinstance(key, Ed25519PrivateKey):
        parser.error(f"{args.key} is not an Ed25519 private key")
    signature = key.sign(canonical_bytes(args.licensee, expires_at))
    licence = {
        "licensee": args.licensee,
        "expires_at": expires_at,
        "signature": base64.b64encode(signature).decode(),
    }
    text = json.dumps(licence, indent=2, ensure_ascii=False) + "\n"
    if args.out:
        args.out.write_text(text, encoding="utf-8")
        print(f"wrote {args.out}: {args.licensee}, expires {expires_at}")
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
