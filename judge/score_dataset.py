"""Loads the evaluation datasets into a server and scores it the way the PRD's test tool would.

    uv run python score_dataset.py                                  all datasets, own server
    uv run python score_dataset.py --target http://127.0.0.1:8000   a running server
    uv run python score_dataset.py datasets/04_map_alerts.json \
        --target http://127.0.0.1:8000 --keep-data          one dataset, left loaded to browse

Each dataset: reset, shift its times to now, post every report and dispute, check every status
code and expected outcome, time the lookups, then reset and check everything is empty and the
licence survived. Scores are points per PRD row times the share of that row's checks that passed.
--keep-data skips the final reset so the data stays for browsing. Never run this against the
server the judges use: it resets it.

--no-load checks a server that already holds the dataset (loaded with curl by curl_dataset.py),
and --summary prints one PRD table from the result files that --json wrote.

Owner: B. Filled in by: B9.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from collections import defaultdict
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from pathlib import Path
from statistics import quantiles
from typing import Any

import httpx

from contract import admin_headers, local_server
from scorecard import NOT_MEASURED, ROWS

HERE = Path(__file__).resolve().parent
TIME_FIELDS = ("occurred_at", "reported_at")
LOOKUP_LIMIT_MS = 500


@dataclass(frozen=True)
class Result:
    row: str
    label: str
    passed: bool
    detail: str = ""


def shift(body: dict[str, Any], delta: timedelta) -> dict[str, Any]:
    """Move the report's times by delta, keeping each one's own offset and format."""
    out = dict(body)
    for field in TIME_FIELDS:
        raw = out.get(field)
        if not isinstance(raw, str):
            continue
        try:
            when = datetime.fromisoformat(raw)
        except ValueError:
            continue
        out[field] = (when + delta).isoformat()
    return out


def masked(display: str) -> bool:
    return "XX" in display or "***" in display or display.endswith("****")


def check_lookup(c: httpx.Client, e: dict[str, Any], timings: list[float]) -> Result:
    started = time.perf_counter()
    response = c.get("/lookup", params={"type": e["type"], "value": e["value"]})
    timings.append((time.perf_counter() - started) * 1000)
    if response.status_code != 200:
        return Result(e["row"], e["label"], False, f"status {response.status_code}")
    body = response.json()
    problems = []
    for field in ("verdict", "distinct_reporters", "report_count", "challenged", "scam_type"):
        if field in e and body[field] != e[field]:
            problems.append(f"{field} {body[field]!r}, want {e[field]!r}")
    if "not_verdict" in e and body["verdict"] == e["not_verdict"]:
        problems.append(f"verdict is {body['verdict']}")
    if e.get("display") == "masked" and not masked(body["display_value"]):
        problems.append("display_value is not masked")
    if e.get("display") == "full" and masked(body["display_value"]):
        problems.append(f"display_value {body['display_value']!r} is still masked")
    return Result(e["row"], e["label"], not problems, "; ".join(problems))


def check(c: httpx.Client, e: dict[str, Any], timings: list[float]) -> Result:
    kind = e["kind"]
    if kind == "lookup":
        return check_lookup(c, e, timings)
    if kind == "lookup_status":
        status = c.get("/lookup", params={"type": e["type"], "value": e["value"]}).status_code
        return Result(e["row"], e["label"], status == e["status"], f"status {status}")
    if kind == "same_answer":
        answers = [
            c.get("/lookup", params={"type": e["type"], "value": v}).json() for v in e["values"]
        ]
        same = all(a == answers[0] for a in answers)
        counts = [a.get("report_count") for a in answers]
        return Result(e["row"], e["label"], same, "" if same else f"report counts {counts}")
    if kind == "map":
        params = {"pincode": e["pincode"]} | {k: str(v) for k, v in e["params"].items()}
        found = {x["pincode"]: x["count"] for x in c.get("/map", params=params).json()["clusters"]}
        problems = [f"{p} should not be on the map" for p in e["absent"] if p in found]
        if e["pincodes"] is not None:
            if set(found) != set(e["pincodes"]):
                problems.append(f"areas {sorted(found)}, want {sorted(e['pincodes'])}")
            problems += [
                f"{p} count {found.get(p)}, want {n}"
                for p, n in e["pincodes"].items()
                if n is not None and p in found and found[p] != n
            ]
        return Result(e["row"], e["label"], not problems, "; ".join(problems))
    if kind == "alerts":
        found = c.get("/alerts", params={"pincode": e["pincode"]}).json()["alerts"]
        spike = any(a["type"] == "SPIKE" for a in found)
        problems = [] if spike == e["spike"] else [f"SPIKE {spike}, want {e['spike']}"]
        if e["new_types"] is not None:
            new = sorted(a["scam_type"] for a in found if a["type"] == "NEW_TYPE")
            if new != sorted(e["new_types"]):
                problems.append(f"NEW_TYPE {new}, want {sorted(e['new_types'])}")
        return Result(e["row"], e["label"], not problems, "; ".join(problems))
    if kind == "severity":
        params = {"pincode": e["pincode"], "radius_km": "0"}
        clusters = c.get("/map", params=params).json()["clusters"]
        got = clusters[0]["severity"] if clusters else None
        return Result(e["row"], e["label"], got == e["severity"], f"severity {got}")
    return Result(e["row"], e["label"], False, f"unknown expectation kind {kind!r}")


def status_results(
    doc: dict[str, Any], reports: dict[str, int], disputes: list[int]
) -> list[Result]:
    """Compare the status of every request with what the dataset expects (201, 400 or 429)."""
    wrong_accept: list[str] = []
    wrong_refuse: list[str] = []
    for body in doc["reports"]:
        status = reports.get(body["report_id"])
        want = doc["expect_status"].get(body["report_id"], [201])
        if status not in want:
            target = wrong_accept if want == [201] else wrong_refuse
            target.append(f"{body['report_id']}: {status}, want {want}")
    refused = [b for b in doc["reports"] if b["report_id"] in doc["expect_status"]]
    results = [
        Result(
            "2",
            f"all {len(doc['reports']) - len(refused)} valid reports accepted",
            not wrong_accept,
            "; ".join(wrong_accept[:3]),
        )
    ]
    if refused:
        results.append(
            Result(
                "1",
                f"all {len(refused)} junk or spam reports refused (400/429)",
                not wrong_refuse,
                "; ".join(wrong_refuse[:3]),
            )
        )
    for n, (challenge, want) in enumerate(
        zip(doc["challenges"], doc["challenge_status"], strict=True)
    ):
        status = disputes[n] if n < len(disputes) else None
        results.append(
            Result(
                "bonus:dispute",
                f"dispute by {challenge['claimant_name']}: {want}",
                status in want,
                f"status {status}",
            )
        )
    return results


def read_status_log(path: Path) -> tuple[dict[str, int], list[int]]:
    """The status of every request, as logged (status, tab, what) by a curl_dataset.py script."""
    reports: dict[str, int] = {}
    disputes: list[int] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        status, _, what = line.partition("\t")
        if what.startswith("dispute "):
            disputes.append(int(status))
        else:
            reports[what] = int(status)
    return reports, disputes


def load_with_python(c: httpx.Client, doc: dict[str, Any], delta: timedelta) -> list[Result]:
    """Reset, then post every report and dispute with the Python client, checking each status."""
    c.post("/admin/reset").raise_for_status()
    reports = {
        body["report_id"]: c.post("/reports", json=shift(body, delta)).status_code
        for body in doc["reports"]
    }
    disputes = [c.post("/challenges", json=ch).status_code for ch in doc["challenges"]]
    return status_results(doc, reports, disputes)


def score_dataset(
    c: httpx.Client,
    path: Path,
    keep_data: bool,
    load: bool = True,
    statuses: Path | None = None,
) -> list[Result]:
    """Load a dataset with the Python client (unless it is already loaded) and check it.

    With load=False the server already holds the dataset, for example loaded with curl by
    curl_dataset.py, so only the expected outcomes are checked and the data is always kept;
    `statuses` is that script's status log, scored the same way as the Python client's replies.
    """
    doc = json.loads(path.read_text(encoding="utf-8"))
    delta = datetime.now(UTC).replace(second=0, microsecond=0) - datetime.fromisoformat(
        doc["_anchor"]
    )
    if load:
        results = load_with_python(c, doc, delta)
    else:
        results = status_results(doc, *read_status_log(statuses)) if statuses else []
        licence = c.get("/license").json()
        results.append(
            Result(
                "5",
                "licence still valid after the reset and the load",
                licence.get("valid") is True,
                str(licence),
            )
        )

    timings: list[float] = []
    results += [check(c, e, timings) for e in doc["expect"]]
    if len(timings) >= 2:
        p95 = quantiles(timings, n=100)[94] if len(timings) > 2 else max(timings)
        results.append(
            Result(
                "2",
                f"lookup p95 under {LOOKUP_LIMIT_MS} ms",
                p95 < LOOKUP_LIMIT_MS,
                f"p95 {p95:.1f} ms over {len(timings)} lookups",
            )
        )

    if load and not keep_data:
        c.post("/admin/reset").raise_for_status()
        leftovers = []
        for e in doc["expect"]:
            if e["kind"] != "lookup":
                continue
            body = c.get("/lookup", params={"type": e["type"], "value": e["value"]}).json()
            if body["report_count"] != 0 or body["challenged"]:
                leftovers.append(e["value"])
        results.append(
            Result(
                "2",
                "reset leaves every lookup empty",
                not leftovers,
                f"still there: {leftovers[:3]}",
            )
        )
        licence = c.get("/license").json()
        results.append(
            Result("5", "licence survives reset", licence.get("valid") is True, str(licence))
        )
    return results


def print_report(name: str, results: list[Result]) -> None:
    passed = sum(r.passed for r in results)
    print(f"\n=== {name}: {passed}/{len(results)} checks passed")
    for r in results:
        if not r.passed:
            print(f"  FAIL [{r.row}] {r.label}: {r.detail}")


def row_scores(results: list[Result]) -> dict[str, tuple[float, int, int]]:
    counts: dict[str, list[int]] = defaultdict(lambda: [0, 0])
    for r in results:
        counts[r.row][0] += r.passed
        counts[r.row][1] += 1
    return {row: (ROWS[row][1] * p / n, p, n) for row, (p, n) in counts.items()}


def print_prd_table(every: list[Result], datasets: int) -> int:
    """The estimate per PRD row and the totals; the exit code is 1 when any check failed."""
    print("\nPRD row                              estimate   checks")
    scores = row_scores(every)
    total = 0.0
    for row, (title, points) in ROWS.items():
        if row in scores:
            got, passed, n = scores[row]
            total += got
            note = f"   ({NOT_MEASURED[row]})" if row in NOT_MEASURED else ""
            print(f"{title:<36} {got:5.1f}/{points:<3} {passed:>4}/{n}{note}")
    possible = sum(points for row, (_, points) in ROWS.items() if row in scores)
    print(f"{'Measured total':<36} {total:5.1f}/{possible}")
    print("Not measured here: rows 6 (15) and 7 (10), judged by people. PRD total: 100 + 7 bonus.")
    failed = [r for r in every if not r.passed]
    print(f"\n{len(every) - len(failed)}/{len(every)} checks passed across {datasets} datasets")
    return 1 if failed else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else None)
    parser.add_argument("datasets", nargs="*", type=Path, help="default: every file in datasets/")
    parser.add_argument("--target", default="local", help="'local' (own server) or a base URL")
    parser.add_argument("--keep-data", action="store_true", help="leave the last dataset loaded")
    parser.add_argument("--json", type=Path, help="also write every result here")
    parser.add_argument(
        "--no-load",
        action="store_true",
        help="the server already holds the dataset (for example loaded with curl): only check it",
    )
    parser.add_argument(
        "--statuses",
        type=Path,
        help="with --no-load and one dataset: the status log the curl script wrote (STATUS_LOG)",
    )
    parser.add_argument(
        "--summary",
        nargs="+",
        type=Path,
        metavar="JSON",
        help="print one PRD table from result files written by --json, then exit",
    )
    args = parser.parse_args(argv)
    if args.summary:
        merged: dict[str, list[Result]] = {}
        for file in args.summary:
            for stem, rows in json.loads(file.read_text(encoding="utf-8")).items():
                merged[stem] = [Result(**row) for row in rows]
        return print_prd_table([r for rows in merged.values() for r in rows], len(merged))
    paths = args.datasets or sorted((HERE / "datasets").glob("*.json"))
    if not paths:
        parser.error("no datasets found; run build_datasets.py first")
    if args.statuses and not (args.no_load and len(paths) == 1):
        parser.error("--statuses needs --no-load and exactly one dataset")

    def run(base: str) -> dict[str, list[Result]]:
        out: dict[str, list[Result]] = {}
        with httpx.Client(headers=admin_headers(), base_url=base, timeout=60) as client:
            for i, path in enumerate(paths):
                keep = args.keep_data and i == len(paths) - 1
                out[path.stem] = score_dataset(
                    client, path, keep, load=not args.no_load, statuses=args.statuses
                )
                print_report(path.stem, out[path.stem])
        return out

    if args.target == "local":
        with local_server() as base:
            by_dataset = run(base)
    else:
        by_dataset = run(args.target.rstrip("/"))

    if args.json:
        args.json.write_text(
            json.dumps({k: [r.__dict__ for r in v] for k, v in by_dataset.items()}, indent=1),
            encoding="utf-8",
        )
    every = [r for results in by_dataset.values() for r in results]
    return print_prd_table(every, len(by_dataset))


if __name__ == "__main__":
    sys.exit(main())
