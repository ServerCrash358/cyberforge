"""Offline: generates the Ed25519 key pair.
The private key goes to ~/.scam-map/license.key, never the repo.
Writes the public key to api-rs/public_key.pem, which is built into the server.

Run once, on your laptop:
    uv run --project judge python scripts/license_keygen.py

Owner: B. Filled in by: B3.
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

REPO = Path(__file__).resolve().parent.parent
DEFAULT_KEY = Path.home() / ".scam-map" / "license.key"
DEFAULT_PUBLIC = REPO / "api-rs" / "public_key.pem"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Generate the licence signing key pair.")
    parser.add_argument("--key", type=Path, default=DEFAULT_KEY, help="private key path")
    parser.add_argument("--public-out", type=Path, default=DEFAULT_PUBLIC, help="public key path")
    parser.add_argument("--force", action="store_true", help="replace an existing private key")
    args = parser.parse_args(argv)

    key_path: Path = args.key.expanduser().resolve()
    if key_path.is_relative_to(REPO):
        print(f"refusing: {key_path} is inside the repository", file=sys.stderr)
        return 2
    if key_path.exists() and not args.force:
        print(f"refusing: {key_path} exists; pass --force to replace it", file=sys.stderr)
        print("replacing it invalidates every licence signed with the old key", file=sys.stderr)
        return 2

    private = Ed25519PrivateKey.generate()
    key_path.parent.mkdir(parents=True, exist_ok=True)
    pem = private.private_bytes(
        serialization.Encoding.PEM,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    )
    fd = os.open(key_path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "wb") as f:
        f.write(pem)

    public = private.public_key().public_bytes(
        serialization.Encoding.PEM, serialization.PublicFormat.SubjectPublicKeyInfo
    )
    args.public_out.parent.mkdir(parents=True, exist_ok=True)
    args.public_out.write_bytes(public)

    print(f"private key: {key_path}  (keep it off git and off the server)")
    print(f"public key:  {args.public_out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
