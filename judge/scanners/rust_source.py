"""String literals in the Rust API's source, for the scanners that read code.

A small tokenizer, enough for this codebase: it skips comments and char literals, reads normal
and raw strings, and stops each file at its `#[cfg(test)]` module, where example identifiers
belong. Doc comments are comments, so they are skipped too.

Owner: B. Filled in by: R5.
"""

from __future__ import annotations

from pathlib import Path

from contract import API

SOURCE = API / "src"
TEST_MODULE = "#[cfg(test)]"
BACKSLASH = chr(92)
ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", BACKSLASH: BACKSLASH, '"': '"', "'": "'"}


def source_files(root: Path = SOURCE) -> list[Path]:
    return sorted(root.rglob("*.rs"))


def _without_tests(text: str) -> str:
    cut = text.find(TEST_MODULE)
    return text if cut < 0 else text[:cut]


def _normal_string(text: str, i: int) -> tuple[str, int]:
    """The string starting after the opening quote at i; returns it and the index after it."""
    out: list[str] = []
    j = i
    while j < len(text):
        c = text[j]
        if c == '"':
            return "".join(out), j + 1
        if c == BACKSLASH and j + 1 < len(text):
            nxt = text[j + 1]
            if nxt == "\n":
                # A line continuation: the newline and the next line's indent are dropped.
                j += 2
                while j < len(text) and text[j] in " \t\n":
                    j += 1
                continue
            if nxt == "u" and text[j + 2 : j + 3] == "{":
                end = text.index("}", j)
                out.append(chr(int(text[j + 3 : end], 16)))
                j = end + 1
                continue
            out.append(ESCAPES.get(nxt, nxt))
            j += 2
            continue
        out.append(c)
        j += 1
    return "".join(out), j


def string_literals(path: Path) -> list[tuple[int, str]]:
    """(line, value) of every string literal outside comments and the test module."""
    text = _without_tests(path.read_text(encoding="utf-8"))
    found: list[tuple[int, str]] = []
    i = 0
    while i < len(text):
        c = text[i]
        if text.startswith("//", i):
            i = text.find("\n", i)
            i = len(text) if i < 0 else i
        elif text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = len(text) if end < 0 else end + 2
        elif c == "r" and (text.startswith('r"', i) or text.startswith("r#", i)):
            hashes = 0
            j = i + 1
            while text[j] == "#":
                hashes += 1
                j += 1
            if text[j] != '"':
                i += 1
                continue
            close = '"' + "#" * hashes
            end = text.index(close, j + 1)
            found.append((text.count("\n", 0, i) + 1, text[j + 1 : end]))
            i = end + len(close)
        elif c == '"':
            value, after = _normal_string(text, i + 1)
            found.append((text.count("\n", 0, i) + 1, value))
            i = after
        elif c == "'":
            # A char literal ('x', '\n', '"') is skipped; a lifetime ('a) is not a literal.
            if text[i + 1 : i + 2] == BACKSLASH:
                i = text.index("'", i + 2) + 1
            elif text[i + 2 : i + 3] == "'":
                i += 3
            else:
                i += 1
        elif c.isalnum() or c == "_":
            # Skip whole identifiers so the r of "for" or "letter" never starts a raw string.
            while i < len(text) and (text[i].isalnum() or text[i] == "_"):
                i += 1
        else:
            i += 1
    return found
