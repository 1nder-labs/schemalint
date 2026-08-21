#!/usr/bin/env python3
"""Validate built-in provider-evidence links; network access is opt-in."""

import argparse
import http.client
import sys
import time
import tomllib
import urllib.error
import urllib.request
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urldefrag, urlparse

ALLOWED_HOSTS = {"developers.openai.com", "platform.claude.com"}
PROFILES = Path(__file__).parents[2] / "crates" / "schemalint" / "profiles"


class Anchors(HTMLParser):
    def __init__(self):
        super().__init__()
        self.values = set()

    def handle_starttag(self, _tag, attrs):
        for key, value in attrs:
            if key in {"id", "name"} and value:
                self.values.add(value)


def evidence_urls():
    for path in sorted(PROFILES.glob("*.toml")):
        with path.open("rb") as profile_file:
            profile = tomllib.load(profile_file)
        for evidence in profile.get("evidence", []):
            for source in evidence.get("sources", []):
                yield path.name, evidence["key"], source["url"]


def offline_errors(rows):
    errors = []
    for profile, key, url in rows:
        parsed = urlparse(url)
        if parsed.scheme != "https" or parsed.hostname not in ALLOWED_HOSTS:
            errors.append(f"{profile} {key}: disallowed provider URL {url}")
        if not parsed.fragment:
            errors.append(f"{profile} {key}: provider URL has no fragment {url}")
    return errors


def fetch(url, attempts=3):
    request = urllib.request.Request(url, headers={"User-Agent": "schemalint-link-audit/1"})
    for attempt in range(attempts):
        try:
            with urllib.request.urlopen(request, timeout=20) as response:
                final_base = urldefrag(response.geturl())[0]
                anchors = Anchors()
                anchors.feed(response.read().decode("utf-8", "replace"))
                return final_base, anchors.values
        except urllib.error.HTTPError as error:
            if error.code == 429 or error.code >= 500:
                if attempt + 1 < attempts:
                    time.sleep(2**attempt)
                    continue
                return "inconclusive", f"transient HTTP {error.code}"
            if error.code in {401, 403}:
                return "inconclusive", f"access denied HTTP {error.code}"
            return "broken", f"HTTP {error.code}"
        except (TimeoutError, urllib.error.URLError, OSError, http.client.HTTPException) as error:
            if attempt + 1 < attempts:
                time.sleep(2**attempt)
                continue
            return "inconclusive", f"could not read response: {error}"
        except Exception as error:
            return "inconclusive", f"could not parse response: {error}"


def audit(url, pages):
    base, fragment = urldefrag(url)
    if base not in pages:
        pages[base] = fetch(base)
    page = pages[base]
    if page[0] in {"broken", "inconclusive"}:
        return page
    final_base, anchors = page
    if final_base.rstrip("/") != base.rstrip("/"):
        return "drift", f"redirected to {final_base}"
    if fragment not in anchors:
        return "broken", f"missing fragment #{fragment}"
    return "valid", "ok"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--network", action="store_true")
    args = parser.parse_args()
    rows = list(evidence_urls())
    errors = offline_errors(rows)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"validated {len(rows)} provider evidence URLs offline")
    if not args.network:
        return 0
    failed = False
    conclusive = 0
    pages = {}
    for profile, key, url in rows:
        status, detail = audit(url, pages)
        print(f"{status}: {profile} {key}: {detail}")
        failed |= status in {"broken", "drift"}
        conclusive += status != "inconclusive"
    if failed:
        return 1
    if conclusive == 0:
        print("no provider evidence URLs could be conclusively checked", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
