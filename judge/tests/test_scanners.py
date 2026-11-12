"""Each scanner catches a planted violation, so a clean scan means something.

Owner: B. Filled in by: B7.
"""

from __future__ import annotations

import json
from pathlib import Path

from scanners import database, hardcode, logs, responses, wording
from scanners.responses import Exchange

REPORT = {
    "reporter_id": "r_7c2d91aa",
    "ip_prefix": "103.21.58",
    "note": "Video call from a man in police uniform about a parcel",
    "identifier_type": "phone",
    "identifier_value": "+91 98765 43210",
}
SENT = Exchange("POST", "/reports", REPORT, 201, '{"report_id": "x"}')


def reply(path: str, body: object, status: int = 200) -> Exchange:
    return Exchange("GET", path, {}, status, json.dumps(body))


def test_hardcoded_number_is_found(tmp_path: Path) -> None:
    (tmp_path / "rules.rs").write_text(
        "// a comment with +919812345678 is not a literal\n"
        "fn verdict(number: &str) -> &'static str {\n"
        '    if number == "+919876543210" { return "SUSPICIOUS"; }\n'
        '    if number == r"1930" { return "VERIFIED_BUSINESS"; }\n'
        '    "NO_REPORTS"\n'
        "}\n"
        "#[cfg(test)]\n"
        'mod tests { const EXAMPLE: &str = "+919000000001"; }\n',
        encoding="utf-8",
    )
    findings = hardcode.scan(tmp_path)
    assert any("rules.rs:3 has a phone number" in f for f in findings), findings
    assert any("'1930'" in f for f in findings), findings
    assert not any("9812345678" in f or "9000000001" in f for f in findings), findings


def test_website_text_is_scanned_but_comments_are_not(tmp_path: Path) -> None:
    page = tmp_path / "Page.tsx"
    page.write_text(
        '// the words "scammer" and "fraud" are banned\n'
        "export const BANNED_WORDS = ['scammer', 'fraud'];\n"
        "export const Title = () => <h1>Fraud alert</h1>;\n",
        encoding="utf-8",
    )
    assert wording._hits(wording._shown_text(page)) == ["fraud"]


def test_real_website_text_is_clean() -> None:
    assert wording.scan([]) == []


def test_real_api_code_is_clean() -> None:
    assert hardcode.scan() == []


def test_reply_echoing_a_reporter_field_is_found() -> None:
    leak = reply(
        "/lookup",
        {
            "verdict": "NO_REPORTS",
            "distinct_reporters": 0,
            "display_value": "98XXXXX210",
            "debug": "r_7c2d91aa",
        },
    )
    assert any("secret" in f for f in responses.scan([SENT, leak]))


def test_unmasked_value_under_three_is_found() -> None:
    body = {
        "verdict": "SUSPICIOUS",
        "distinct_reporters": 2,
        "display_value": "+919876543210",
        "evidence": [],
    }
    assert any("unmasked" in f for f in responses.scan([reply("/lookup", body)]))


def test_map_point_off_appendix_b_is_found() -> None:
    body = {"clusters": [{"pincode": "560102", "lat": 12.91, "lng": 77.64}]}
    assert any("Appendix B" in f for f in responses.scan([reply("/map", body)]))


def test_log_line_with_a_query_string_is_found(tmp_path: Path) -> None:
    log = tmp_path / "server.log"
    log.write_text("INFO: GET /lookup?type=phone&value=9876543210\n", encoding="utf-8")
    findings = logs.scan(log, [SENT])
    assert any("query string" in f for f in findings)
    assert any("phone number" in f for f in findings)


def test_clean_log_passes(tmp_path: Path) -> None:
    log = tmp_path / "server.log"
    log.write_text("INFO:     Application startup complete.\n", encoding="utf-8")
    assert logs.scan(log, [SENT]) == []


def test_database_holding_a_note_or_raw_number_is_found(tmp_path: Path) -> None:
    (tmp_path / "scam_map.sqlite").write_bytes(
        b"\x00\x01" + REPORT["note"].encode() + b"\x00" + b"9876543210" + b"\x02"
    )
    findings = database.scan(tmp_path, [SENT])
    assert any("note" in f for f in findings)
    assert any("raw identifier" in f for f in findings)


def test_banned_word_in_a_reply_is_found() -> None:
    bad = reply("/lookup", {"error": "this number belongs to a fraud"}, status=400)
    assert any("fraud" in f for f in wording.scan([bad]))
