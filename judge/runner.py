"""Posts scenarios to a base URL or the in-process app and collects replies (--seeds, --target).

Each scenario starts from POST /admin/reset, so scenarios never see each other's data. That
also means: never point this at the server after the judges have loaded their data.

    uv run python runner.py --seeds tuning --target local
    uv run python runner.py --seeds holdout --target https://host --fail-under-thresholds
    uv run python runner.py --replay T1:105 --salt 3f9a... --target local

Owner: B. Filled in by: B4.
"""

from __future__ import annotations

import argparse
import json
import secrets
import sys
import time
import tomllib
from datetime import UTC, datetime
from pathlib import Path

from contract import Client, Response, admin_headers, local_server
from families import FAMILIES
from generator import Gen, Outcome, Scenario
from scorecard import CheckResult, ScenarioResult, failing_thresholds, render

HERE = Path(__file__).resolve().parent


def load_seeds(name: str) -> list[int]:
    if name in ("tuning", "holdout"):
        seeds: list[int] = tomllib.loads((HERE / "seeds.toml").read_text(encoding="utf-8"))[name]
        return seeds
    return [int(x) for x in name.split(",")]


def run_scenario(client: Client, scenario: Scenario) -> ScenarioResult:
    outcome = Outcome()
    try:
        _expect_ok(client.post("/admin/reset").status_code, "reset")
        for step in scenario.reports:
            response = client.post("/reports", json=step.body)
            outcome.statuses.append(response.status_code)
            outcome.replies.append(_json_or_none(response))
        for challenge in scenario.challenges:
            outcome.challenge_statuses.append(
                client.post("/challenges", json=challenge).status_code
            )
        if scenario.reset_before_checks:
            _expect_ok(client.post("/admin/reset").status_code, "reset")
    except Exception as e:  # any transport failure fails every check of this scenario
        detail = f"setup failed: {type(e).__name__}: {e}"
        return ScenarioResult.from_checks(
            scenario, [CheckResult(c.row, c.label, False, detail) for c in scenario.checks]
        )

    results = []
    for check in scenario.checks:
        try:
            check.fn(client, outcome)
        except AssertionError as e:
            results.append(CheckResult(check.row, check.label, False, str(e)))
        except Exception as e:  # a crash in a check is a failed check, not a crashed run
            results.append(CheckResult(check.row, check.label, False, f"{type(e).__name__}: {e}"))
        else:
            results.append(CheckResult(check.row, check.label, True, ""))
    return ScenarioResult.from_checks(scenario, results)


def _json_or_none(response: Response) -> object:
    try:
        body: object = response.json()
        return body
    except ValueError:
        return None


def _expect_ok(status: int, what: str) -> None:
    if status != 200:
        raise RuntimeError(f"{what} returned {status}")


def run_all(
    client: Client, seeds: list[int], families: list[str], salt: str
) -> list[ScenarioResult]:
    results = []
    for seed in seeds:
        for family in families:
            gen = Gen(family, seed, salt, datetime.now(UTC))
            result = run_scenario(client, FAMILIES[family](gen))
            results.append(result)
            print("." if result.passed else "F", end="", flush=True)
    print()
    return results


def main(argv: list[str] | None = None) -> int:
    import httpx

    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else None)
    parser.add_argument("--target", required=True, help="'local' or a base URL")
    parser.add_argument("--seeds", default="tuning", help="tuning, holdout, or 1,2,3")
    parser.add_argument("--families", default=",".join(FAMILIES), help="for example T1,R1")
    parser.add_argument("--salt", default=None, help="replay a run's identifiers exactly")
    parser.add_argument("--replay", default=None, help="FAMILY:SEED, needs --salt")
    parser.add_argument("--fail-under-thresholds", action="store_true")
    parser.add_argument("--json", type=Path, default=None, help="also write results here")
    args = parser.parse_args(argv)

    salt = args.salt or secrets.token_hex(4)
    families = [f.strip().upper() for f in args.families.split(",")]
    seeds = load_seeds(args.seeds)
    if args.replay:
        family, _, seed = args.replay.partition(":")
        families, seeds = [family.upper()], [int(seed)]
    unknown = [f for f in families if f not in FAMILIES]
    if unknown:
        parser.error(f"unknown families: {unknown}")

    started = time.monotonic()

    def go(client: httpx.Client, target: str) -> int:
        print(f"{len(seeds)} seeds x {len(families)} families against {target}, salt {salt}")
        results = run_all(client, seeds, families, salt)
        print(render(results, seeds_name=args.seeds, salt=salt, seconds=time.monotonic() - started))
        if args.json:
            args.json.write_text(
                json.dumps([r.to_dict() for r in results], indent=2), encoding="utf-8"
            )
        if args.fail_under_thresholds and failing_thresholds(results):
            return 1
        return 0

    if args.target == "local":
        with (
            local_server() as base,
            httpx.Client(headers=admin_headers(), base_url=base, timeout=30) as client,
        ):
            return go(client, f"local ({base})")
    with httpx.Client(
        headers=admin_headers(), base_url=args.target.rstrip("/"), timeout=30
    ) as client:
        return go(client, args.target)


if __name__ == "__main__":
    sys.exit(main())
