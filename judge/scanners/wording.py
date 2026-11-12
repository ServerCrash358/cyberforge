"""Banned words ('scammer', 'fraud') absent from API messages and from everything shown to users.

Scans every reply from a run, the API's strings in api-rs/src, the shared data shown in
the app, and the website's source in ui/src (its dictionary is also tested in ui/src).

Owner: B. Filled in by: B7, R5 (Rust source), D1 (React website).
"""

from __future__ import annotations

from pathlib import Path

from contract import REPO
from scanners.responses import Exchange
from scanners.rust_source import source_files, string_literals

BANNED = ("scammer", "fraud", "धोखेबाज", "ठग", "फ्रॉड")
SHOWN_DATA = sorted((REPO / "shared" / "data").glob("*.json"))
WEBSITE = REPO / "ui" / "src"
# The website's own list of banned words, which the website tests against.
BANNED_LIST = "export const BANNED_WORDS"


def _hits(text: str) -> list[str]:
    lowered = text.lower()
    return [word for word in BANNED if word in lowered]


def _website_files() -> list[Path]:
    """Everything the website can show: its source, without its tests."""
    found = [*WEBSITE.rglob("*.ts"), *WEBSITE.rglob("*.tsx")]
    return sorted(p for p in found if ".test." not in p.name)


def _shown_text(path: Path) -> str:
    """The file without comments and without the banned-word list: what can reach a screen."""
    lines = path.read_text(encoding="utf-8").splitlines()
    shown = [
        line
        for line in lines
        if not line.startswith(BANNED_LIST) and not line.lstrip().startswith(("//", "/*", "*"))
    ]
    return "\n".join(shown)


def scan(log: list[Exchange]) -> list[str]:
    findings = [f"{x.method} {x.path} reply says {word!r}" for x in log for word in _hits(x.text)]
    for path in source_files():
        for line, literal in string_literals(path):
            findings += [f"{path.relative_to(REPO)}:{line} has {w!r}" for w in _hits(literal)]
    for path in [*SHOWN_DATA, *_website_files()]:
        findings += [f"{path.relative_to(REPO)} shows {w!r}" for w in _hits(_shown_text(path))]
    return findings
