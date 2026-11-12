"""Runs every scanner and exits non-zero on any finding (python -m scanners.all --target).

With --target local it starts its own server, runs the simulator through a recording client,
then scans the replies, the server's whole log and the raw database files. Against a deployed
address only the replies can be scanned (and the run resets that server).

Owner: B. Filled in by: B7.
"""

from __future__ import annotations

import argparse
import secrets
import sys
import tempfile
from datetime import UTC, datetime
from pathlib import Path

import httpx

from contract import admin_headers, local_server
from families import FAMILIES
from generator import Gen
from runner import load_seeds, run_scenario
from scanners import database, hardcode, logs, responses, wording

SEEDS = "101,102,103,104,105"


def run_traffic(recorder: responses.Recorder, seeds: list[int], salt: str) -> None:
    for seed in seeds:
        for family in FAMILIES:
            run_scenario(recorder, FAMILIES[family](Gen(family, seed, salt, datetime.now(UTC))))


def report(results: dict[str, list[str]]) -> int:
    failed = False
    for name, findings in results.items():
        print(f"{'FAIL' if findings else 'PASS'}  {name}: {len(findings)} findings")
        for f in findings[:10]:
            print(f"      {f}")
        failed |= bool(findings)
    return 1 if failed else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else None)
    parser.add_argument("--target", required=True, help="'local' or a base URL")
    parser.add_argument("--seeds", default=SEEDS, help="seeds for the traffic run")
    args = parser.parse_args(argv)
    seeds, salt = load_seeds(args.seeds), secrets.token_hex(4)

    results: dict[str, list[str]] = {"hardcoding (api-rs)": hardcode.scan()}
    if args.target == "local":
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
            folder = Path(tmp)
            with (
                local_server(folder) as base,
                httpx.Client(headers=admin_headers(), base_url=base, timeout=30) as http,
            ):
                recorder = responses.Recorder(http)
                run_traffic(recorder, seeds, salt)
            exchanges = len(recorder.log)
            results["responses"] = responses.scan(recorder.log)
            results["logs"] = logs.scan(folder / "server.log", recorder.log)
            results["database"] = database.scan(folder, recorder.log)
            results["wording"] = wording.scan(recorder.log)
    else:
        with httpx.Client(
            headers=admin_headers(), base_url=args.target.rstrip("/"), timeout=30
        ) as http:
            recorder = responses.Recorder(http)
            run_traffic(recorder, seeds, salt)
        exchanges = len(recorder.log)
        results["responses"] = responses.scan(recorder.log)
        results["wording"] = wording.scan(recorder.log)
        print("logs and database are scanned only with --target local")
    print(f"{exchanges} requests scanned (seeds {args.seeds}, salt {salt})")
    return report(results)


if __name__ == "__main__":
    sys.exit(main())
