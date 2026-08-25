#!/usr/bin/env python3
"""Fail closed when the assembled Pages artifact is incomplete or leaks references."""

from __future__ import annotations

import sys
from pathlib import Path

REQUIRED = (
    "index.html",
    "architecture/index.html",
    "roadmap/index.html",
    "changelog/index.html",
    "api/openbim_dt/index.html",
    "search/search_index.json",
    ".nojekyll",
)
FORBIDDEN_SUFFIXES = {
    ".pdf",
    ".xls",
    ".xlsx",
    ".xsd",
    ".rdf",
    ".xslt",
}
ALLOWED_XML = {"sitemap.xml"}


def fail(message: str) -> None:
    print(f"documentation artifact check failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: check-docs-site.py SITE_DIR")

    site = Path(sys.argv[1]).resolve()
    if not site.is_dir():
        fail(f"site directory does not exist: {site}")

    missing = [relative for relative in REQUIRED if not (site / relative).is_file()]
    if missing:
        fail("missing required outputs: " + ", ".join(missing))

    forbidden = sorted(
        path.relative_to(site).as_posix()
        for path in site.rglob("*")
        if path.is_file()
        and (
            path.suffix.lower() in FORBIDDEN_SUFFIXES
            or (path.suffix.lower() == ".xml" and path.name not in ALLOWED_XML)
        )
    )
    if forbidden:
        fail("restricted-looking artifacts were published: " + ", ".join(forbidden))

    html = "\n".join(
        (site / relative).read_text(encoding="utf-8")
        for relative in (
            "index.html",
            "architecture/index.html",
            "roadmap/index.html",
            "changelog/index.html",
        )
    )
    for marker in ("ISO 23387", "openbim-dt", "0.1.1"):
        if marker not in html:
            fail(f"canonical documentation marker is absent: {marker}")

    print(f"documentation artifact verified: {site}")


if __name__ == "__main__":
    main()
