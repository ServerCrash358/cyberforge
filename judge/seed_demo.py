"""Fills a server with realistic demo data and prints what to try on the website.

    uv run python seed_demo.py --target http://127.0.0.1:8000

Resets the server first. Uses the simulator's own scenarios, so the data shows exactly what the
trust rules do: a real campaign becomes a known scam, 40 friends bombing a bakery do not, one area
has a spike, another reported suspicious activity. Never point it at the server the judges use.

Owner: B. Filled in by: B9.
"""

from __future__ import annotations

import argparse
import sys
from datetime import UTC, datetime, timedelta
from typing import Any

import httpx

from contract import admin_headers
from families import r3_small, t1_bombing
from families._shared import area_traffic, real_reports, steady_days
from generator import Gen, Scenario, areas, scam_types

HSR, KORAMANGALA, INDIRANAGAR = "560102", "560034", "560038"
NEW_KIND = "sextortion"


def post_all(client: httpx.Client, scenario: Scenario) -> tuple[int, int]:
    ok = refused = 0
    for step in scenario.reports:
        if client.post("/reports", json=step.body).status_code == 201:
            ok += 1
        else:
            refused += 1
    return ok, refused


def lookup(client: httpx.Client, kind: str, value: str) -> dict[str, Any]:
    result: dict[str, Any] = client.get("/lookup", params={"type": kind, "value": value}).json()
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else None)
    parser.add_argument("--target", default="http://127.0.0.1:8000")
    args = parser.parse_args(argv)
    now = datetime.now(UTC)
    tries: list[tuple[str, str, str]] = []

    with httpx.Client(
        headers=admin_headers(), base_url=args.target.rstrip("/"), timeout=30
    ) as client:
        if client.post("/admin/reset").status_code != 200:
            print("reset failed: is the API running and licensed?")
            return 1

        # A real campaign: unrelated people over a week, in HSR Layout and nearby.
        g = Gen("demo-campaign", 1, "demo", now)
        campaign = g.scenario("real campaign")
        number = g.mobile()
        real_reports(
            g,
            campaign,
            "phone",
            f"+91{number}",
            12,
            span=timedelta(days=6),
            pincodes=[HSR, KORAMANGALA],
            scam_type="digital-arrest",
        )
        post_all(client, campaign)
        tries.append(("phone", f"{number[:5]} {number[5:]}", "real campaign by 12 people"))

        # 40 friends bombing a bakery's real number: stays Suspicious and masked.
        bomb = t1_bombing.generate(Gen("T1", 3, "demo", now))
        post_all(client, bomb)
        bakery = bomb.reports[0].body["identifier_value"]
        tries.append(("phone", bakery, "40 friends bombing a bakery"))

        # One or two people: Suspicious and masked.
        small = r3_small.generate(Gen("R3", 2, "demo", now))
        post_all(client, small)
        first = small.reports[0].body
        tries.append((first["identifier_type"], first["identifier_value"], "1 or 2 reporters"))

        # A UPI ID and a fake link reported by several people.
        for kind, seed in (("upi", 4), ("url", 5)):
            g = Gen(f"demo-{kind}", seed, "demo", now)
            s = g.scenario(kind)
            _, raw, normalised = g.identifier(kind)
            real_reports(g, s, kind, normalised, 5, span=timedelta(days=3), pincodes=[INDIRANAGAR])
            post_all(client, s)
            tries.append((kind, raw, "reported by 5 people"))

        # Steady traffic around Bengaluru, a spike in Koramangala, suspicious activity in HSR.
        g = Gen("demo-areas", 6, "demo", now)
        traffic = g.scenario("traffic")
        # Each area has its own mix of scam types, as real areas do. Sextortion is kept out of the
        # mix so it can show up as HSR Layout's reported suspicious activity, and Koramangala's
        # spike reuses types it has already seen so it raises a SPIKE without a NEW_TYPE as well.
        background = [t for t in scam_types() if t != NEW_KIND]
        bengaluru = sorted(p for p, a in areas().items() if a["city"] == "Bengaluru")
        mixes: dict[str, list[str]] = {}
        for pincode in bengaluru:
            mixes[pincode] = g.rng.sample(background, g.rng.randint(2, 4))
            for hours in steady_days(g, 1):
                area_traffic(g, traffic, pincode, [hours], g.rng.choice(mixes[pincode]))
        for _ in range(9):
            hours = g.rng.uniform(1, 20)
            area_traffic(g, traffic, KORAMANGALA, [hours], g.rng.choice(mixes[KORAMANGALA]))
        post_all(client, traffic)
        new_type = g.scenario("new type")
        area_traffic(g, new_type, HSR, [3.0], NEW_KIND)
        post_all(client, new_type)

        print("Demo data loaded. Try these on the Check screen:\n")
        for kind, value, what in tries:
            body = lookup(client, kind, value)
            people = body["distinct_reporters"]
            who = "person" if people == 1 else "people"
            print(f"  {value:<34} {body['verdict']:<18} {people:>2} {who}  ({what})")
        print("  1930                               VERIFIED_BUSINESS        (helpline)")
        print("  12345                              rejected as junk")
        print("\nAlerts on the Map screen (read back from the API):")
        for pincode in sorted(areas()):
            for alert in client.get("/alerts", params={"pincode": pincode}).json()["alerts"]:
                if alert["type"] == "SPIKE":
                    detail = f"{alert['count']} today, about {alert['baseline_per_day']}/day before"
                else:
                    detail = f"reported suspicious activity: {alert['scam_type']}"
                print(f"  {areas()[pincode]['locality']:<16} {alert['type']:<9} {detail}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
