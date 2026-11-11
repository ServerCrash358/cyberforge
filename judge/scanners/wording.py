"""Banned words ('scammer', 'fraud') absent from API messages and from everything shown to users.

Scans every reply from a run, the API's strings in api-rs/src, the shared data shown in
the app, and the web app's dictionaries (also tested in web/tests, A3).

Owner: B. Filled in by: B7, R5 (Rust source).
"""

from __future__ import annotations

import ast
from pathlib import Path

from contract import REPO
from scanners.responses import Exchange
from scanners.rust_source import source_files, string_literals

BANNED = ("scammer", "fraud", "धोखेबाज", "ठग", "फ्रॉड")
SHOWN_FILES = [
    *(REPO / "shared" / "data").glob("*.json"),
    REPO / "web" / "scam_web" / "i18n.py",
]


def _string_literals(path: Path) -> list[str]:
    """String constants in a Python file, minus docstrings and the banned-word list itself."""
    tree = ast.parse(path.read_text(encoding="utf-8"))
    skip: set[int] = set()
    for node in ast.walk(tree):
        body = getattr(node, "body", None)
        if isinstance(body, list) and body and isinstance(body[0], ast.Expr):
            skip.add(id(body[0].value))
        if isinstance(node, ast.Assign) and any(
            isinstance(t, ast.Name) and t.id in ("BANNED_WORDS", "BANNED") for t in node.targets
        ):
            skip.update(id(n) for n in ast.walk(node.value))
    return [
        n.value
        for n in ast.walk(tree)
        if isinstance(n, ast.Constant) and isinstance(n.value, str) and id(n) not in skip
    ]


def _hits(text: str) -> list[str]:
    lowered = text.lower()
    return [word for word in BANNED if word in lowered]


def scan(log: list[Exchange]) -> list[str]:
    findings = [f"{x.method} {x.path} reply says {word!r}" for x in log for word in _hits(x.text)]
    for path in source_files():
        for line, literal in string_literals(path):
            findings += [f"{path.relative_to(REPO)}:{line} has {w!r}" for w in _hits(literal)]
    for path in SHOWN_FILES:
        texts = (
            _string_literals(path) if path.suffix == ".py" else [path.read_text(encoding="utf-8")]
        )
        for text in texts:
            findings += [f"{path.relative_to(REPO)} shows {w!r}" for w in _hits(text)]
    return findings
