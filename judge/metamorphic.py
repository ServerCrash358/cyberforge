"""Metamorphic tests: answers must not depend on things that should not matter.

Each scenario is run as generated, then again with one change that must leave every verdict,
count and display value identical: reports shuffled, identifiers re-spelled, times written
with other offsets, or junk mixed in. (Notes switched between languages is left for later.)
Families whose expected statuses depend on order (T3's 429s, A3's spammer) are not used.

    uv run python metamorphic.py --seeds tuning --target local

Owner: B. Filled in by: B5.
"""

from __future__ import annotations

import argparse
import random
import re
import secrets
import sys
from dataclasses import replace
from datetime import UTC, datetime, timedelta, timezone
from typing import Any

import httpx

from contract import Client, admin_headers, local_server
from families import FAMILIES
from generator import Gen, ReportStep, Scenario, lookup, phone_formats
from runner import load_seeds

FAMILIES_USED = ["T1", "T2", "T5", "R1", "R2", "R3", "F1", "C1"]
TRANSFORMS = ["shuffled", "respelled", "other offsets", "junk added"]
OFFSETS = [UTC, timezone(timedelta(hours=-5)), timezone(timedelta(hours=9, minutes=30))]
JUNK: list[dict[str, Any]] = [
    {"identifier_value": "12345"},
    {"identifier_value": "test"},
    {"pincode": "999999"},
    {"occurred_at": "yesterday"},
]


def respell(kind: str, value: str) -> str:
    """Another way of typing the same identifier."""
    if kind == "phone":
        digits = "".join(ch for ch in value if ch.isdigit())
        if digits.startswith(("1800", "1860")) or len(digits) < 10:
            # Toll-free and short helplines keep every digit; only the spacing changes.
            spaced = f"{digits[:4]}-{digits[4:]}" if len(digits) > 4 else f" {digits} "
            return spaced if spaced != value else digits
        spellings = [s for s in phone_formats(digits[-10:]) if s != value]
        return spellings[len(digits) % len(spellings)]
    if kind == "upi":
        return f" {value.upper()} "
    if kind == "url":
        bare = value.strip().split("://", 1)[-1].split("?", 1)[0].split("#", 1)[0]
        bare = bare.removeprefix("www.").removeprefix("WWW.").rstrip("/")
        return f"http://www.{bare}/?utm=metamorphic"
    code = re.sub(r"^[A-Za-z]{2}-", "", value.strip()).upper()
    return f"JD-{code.lower()}" if value.strip().upper() == code else code


def transform(scenario: Scenario, name: str, seed: int) -> list[ReportStep]:
    rng = random.Random(f"{scenario.family}:{scenario.seed}:{name}:{seed}")
    steps = list(scenario.reports)
    if name == "shuffled":
        rng.shuffle(steps)
    elif name == "respelled":
        steps = [
            replace(
                s,
                body=s.body
                | {
                    "identifier_value": respell(
                        s.body["identifier_type"], s.body["identifier_value"]
                    )
                },
            )
            for s in steps
        ]
    elif name == "other offsets":
        tz = rng.choice(OFFSETS)
        steps = [
            replace(
                s,
                body=s.body
                | {
                    f: datetime.fromisoformat(s.body[f]).astimezone(tz).isoformat()
                    for f in ("occurred_at", "reported_at")
                },
            )
            for s in steps
        ]
    elif name == "junk added":
        for n, change in enumerate(JUNK):
            base = rng.choice(scenario.reports).body
            junk = base | change | {"report_id": f"{base['report_id']}_junk{n}"}
            steps.insert(rng.randrange(len(steps) + 1), ReportStep(junk, frozenset({400})))
    return steps


def answers(client: Client, scenario: Scenario, steps: list[ReportStep]) -> dict[str, Any]:
    """Post the steps on a clean server, then read the answer for the scenario's identifier."""
    if client.post("/admin/reset").status_code != 200:
        raise RuntimeError("reset failed")
    for step in steps:
        client.post("/reports", json=step.body)
    for challenge in scenario.challenges:
        client.post("/challenges", json=challenge)
    first = scenario.reports[0].body
    body = lookup(client, first["identifier_type"], first["identifier_value"])
    keep = ("verdict", "report_count", "distinct_reporters", "display_value", "challenged")
    return {k: body[k] for k in keep}


def run(client: Client, seeds: list[int], salt: str) -> list[str]:
    failures = []
    for seed in seeds:
        for family in FAMILIES_USED:
            scenario = FAMILIES[family](Gen(family, seed, salt, datetime.now(UTC)))
            try:
                expected = answers(client, scenario, scenario.reports)
            except (httpx.HTTPError, AssertionError, RuntimeError) as e:
                failures += [f"{family}:{seed} {n}: baseline failed: {e!r}" for n in TRANSFORMS]
                print("E" * len(TRANSFORMS), end="", flush=True)
                continue
            for name in TRANSFORMS:
                try:
                    got = answers(client, scenario, transform(scenario, name, seed))
                except (httpx.HTTPError, AssertionError, RuntimeError) as e:
                    failures.append(f"{family}:{seed} {name}: request failed: {e!r}")
                    print("E", end="", flush=True)
                    continue
                ok = got == expected
                print("." if ok else "F", end="", flush=True)
                if not ok:
                    diff = {k: (expected[k], got[k]) for k in expected if expected[k] != got[k]}
                    failures.append(f"{family}:{seed} {name}: {diff}")
    print()
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else None)
    parser.add_argument("--target", required=True, help="'local' or a base URL")
    parser.add_argument("--seeds", default="tuning", help="tuning, holdout, or 1,2,3")
    parser.add_argument("--salt", default=None)
    args = parser.parse_args(argv)
    salt = args.salt or secrets.token_hex(4)
    seeds = load_seeds(args.seeds)

    def go(client: httpx.Client) -> int:
        failures = run(client, seeds, salt)
        total = len(seeds) * len(FAMILIES_USED) * len(TRANSFORMS)
        print(f"{total - len(failures)}/{total} metamorphic checks identical (salt {salt})")
        for f in failures[:25]:
            print("  " + f)
        return 1 if failures else 0

    if args.target == "local":
        with (
            local_server() as base,
            httpx.Client(headers=admin_headers(), base_url=base, timeout=30) as client,
        ):
            return go(client)
    with httpx.Client(
        headers=admin_headers(), base_url=args.target.rstrip("/"), timeout=30
    ) as client:
        return go(client)


if __name__ == "__main__":
    sys.exit(main())
