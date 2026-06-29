"""Shared environment loader for validation scripts.

Reads a ``.env`` file from this directory into ``os.environ`` using python-dotenv
when available, falling back to a minimal hand-rolled parser that handles:

  - blank lines (skipped)
  - comment lines starting with ``#`` (skipped)
  - optional ``export `` prefix (stripped)
  - ``KEY=value`` pairs where the value may itself contain ``=``
  - single- or double-quoted values (quotes stripped, inline ``#`` inside
    quotes is preserved as literal content)
  - unquoted values: trailing inline comments (`` #...``) are stripped
  - existing env vars are *not* overwritten (``setdefault`` semantics)
"""

import os
from pathlib import Path


def _parse_dotenv_value(raw: str) -> str:
    """Return the logical value from the raw RHS of a dotenv KEY=<raw> line."""
    val = raw.strip()
    if not val:
        return val
    # Quoted value: strip matching outer quotes; content is literal.
    if (val.startswith('"') and val.endswith('"')) or (
        val.startswith("'") and val.endswith("'")
    ):
        return val[1:-1]
    # Unquoted value: strip trailing inline comment (`` #`` or ``\t#``).
    comment_idx = val.find(" #")
    if comment_idx == -1:
        comment_idx = val.find("\t#")
    if comment_idx != -1:
        val = val[:comment_idx]
    return val.rstrip()


def load_env() -> None:
    env_path = Path(__file__).resolve().parent / ".env"
    if not env_path.exists():
        return
    try:
        from dotenv import load_dotenv

        load_dotenv(env_path)
    except ImportError:
        with open(env_path) as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                # Strip optional leading ``export ``
                if line.startswith("export "):
                    line = line[len("export "):]
                if "=" not in line:
                    continue
                key, _, raw_val = line.partition("=")
                key = key.strip()
                if not key:
                    continue
                os.environ.setdefault(key, _parse_dotenv_value(raw_val))
