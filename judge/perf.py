"""Latency over HTTPS from outside the host: 100k reports, 2,000 lookups, 500 reports, cold start.

    uv run python perf.py --target https://host            (resets the server first)
    uv run python perf.py --target local --reports 100000
    uv run python perf.py --target https://host --idle-minutes 15   (one request after idling)

Fails when lookup p95 is over --lookup-p95-ms (300 by default, SPEC test layer 7).

Owner: B. Filled in by: B8.
"""

from __future__ import annotations

import argparse
import random
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from datetime import UTC, datetime, timedelta
from statistics import quantiles
from typing import Any

import httpx

from contract import admin_headers, local_server
from generator import IST, areas, channels, scam_types

NOTE_WORDS = [
    *("call", "video", "police", "parcel", "customs", "kyc", "bank", "link", "otp", "refund"),
    *("qr", "job", "deposit", "task", "loan", "app", "threat", "power", "bill", "lottery"),
    *("prize", "uniform", "arrest", "money", "transfer", "screen", "share", "asked", "said"),
]


def report_body(rng: random.Random, n: int, numbers: list[str], now: datetime) -> dict[str, Any]:
    occurred = now - timedelta(minutes=rng.randint(30, 60 * 24 * 30))
    note = " ".join(rng.choices(NOTE_WORDS, k=rng.randint(4, 18))) + f" {rng.randint(1, 10**6)}"
    return {
        "report_id": f"perf_{n}",
        "occurred_at": occurred.astimezone(IST).replace(microsecond=0).isoformat(),
        "reported_at": (occurred + timedelta(minutes=rng.randint(2, 600)))
        .astimezone(IST)
        .replace(microsecond=0)
        .isoformat(),
        "reporter_id": f"perf_r_{n}",
        "ip_prefix": f"{rng.randint(1, 223)}.{rng.randint(0, 255)}.{rng.randint(0, 255)}",
        "channel": rng.choice(channels()),
        "scam_type": rng.choice(scam_types()),
        "identifier_type": "phone",
        "identifier_value": rng.choice(numbers),
        "pincode": rng.choice(sorted(areas())),
        "note": note,
    }


def timed(client: httpx.Client, method: str, path: str, **kwargs: Any) -> tuple[float, int]:
    started = time.perf_counter()
    response = client.request(method, path, **kwargs)
    return (time.perf_counter() - started) * 1000, response.status_code


def summary(name: str, samples: list[float]) -> str:
    cuts = quantiles(samples, n=100)
    return (
        f"{name:<8} n={len(samples):<6} p50 {cuts[49]:7.1f} ms   p95 {cuts[94]:7.1f} ms"
        f"   max {max(samples):7.1f} ms"
    )


def run(base: str, args: argparse.Namespace) -> int:
    rng = random.Random(args.seed)
    now = datetime.now(UTC)
    numbers = [rng.choice("6789") + f"{rng.randint(0, 10**9 - 1):09d}" for _ in range(args.numbers)]
    limits = httpx.Limits(max_connections=args.workers, max_keepalive_connections=args.workers)
    with httpx.Client(headers=admin_headers(), base_url=base, timeout=60, limits=limits) as client:
        if args.reports:
            client.post("/admin/reset").raise_for_status()
            started = time.monotonic()
            bodies = (report_body(rng, n, numbers, now) for n in range(args.reports))
            with ThreadPoolExecutor(max_workers=args.workers) as pool:
                statuses = list(
                    pool.map(lambda b: client.post("/reports", json=b).status_code, bodies)
                )
            took = time.monotonic() - started
            bad = len([s for s in statuses if s != 201])
            rate = args.reports / took
            print(f"loaded {args.reports} reports in {took:.0f}s ({rate:.0f}/s), {bad} refused")

        lookups = []
        for _ in range(args.lookups):
            value = (
                rng.choice(numbers) if rng.random() < 0.8 else f"9{rng.randint(10**8, 10**9 - 1)}"
            )
            ms, status = timed(client, "GET", "/lookup", params={"type": "phone", "value": value})
            if status != 200:
                print(f"lookup returned {status}")
                return 1
            lookups.append(ms)

        writes = []
        for n in range(args.writes):
            body = report_body(rng, args.reports + n, numbers, now)
            writes.append(timed(client, "POST", "/reports", json=body)[0])

        maps = []
        for _ in range(100):
            pin = rng.choice(sorted(areas()))
            params = {"pincode": pin, "days": "7", "radius_km": "5"}
            maps.append(timed(client, "GET", "/map", params=params)[0])

        print(summary("lookup", lookups))
        print(summary("report", writes))
        print(summary("map", maps))

        if args.idle_minutes:
            print(f"idling {args.idle_minutes} minutes for the cold-start check")
            time.sleep(args.idle_minutes * 60)
            params = {"type": "phone", "value": numbers[0]}
            ms, _ = timed(client, "GET", "/lookup", params=params)
            print(f"first lookup after idling: {ms:.0f} ms")
            if ms > 1000:
                print("FAIL: cold start over 1 s")
                return 1

    p95 = quantiles(lookups, n=100)[94]
    if p95 > args.lookup_p95_ms:
        print(f"FAIL: lookup p95 {p95:.0f} ms is over {args.lookup_p95_ms:.0f} ms")
        return 1
    print(f"PASS: lookup p95 {p95:.1f} ms (limit {args.lookup_p95_ms:.0f} ms)")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else None)
    parser.add_argument("--target", required=True, help="'local' or a base URL")
    parser.add_argument("--reports", type=int, default=100_000, help="0 keeps existing data")
    parser.add_argument("--numbers", type=int, default=20_000, help="distinct phone numbers")
    parser.add_argument("--lookups", type=int, default=2000)
    parser.add_argument("--writes", type=int, default=500)
    parser.add_argument("--workers", type=int, default=16)
    parser.add_argument("--idle-minutes", type=float, default=0)
    parser.add_argument("--lookup-p95-ms", type=float, default=300)
    parser.add_argument("--seed", type=int, default=8)
    args = parser.parse_args(argv)
    if args.target == "local":
        with local_server() as base:
            return run(base, args)
    return run(args.target.rstrip("/"), args)


if __name__ == "__main__":
    sys.exit(main())
