"""Shared environment loader for validation scripts.

Reads a ``.env`` file from this directory into ``os.environ`` using python-dotenv
when available, falling back to a minimal hand-rolled parser that handles:

  - blank lines (skipped)
  - comment lines starting with ``#`` (skipped)
  - ``KEY=value`` pairs, with optional surrounding single or double quotes on
    the value side (stripped)
  - existing env vars are *not* overwritten (``setdefault`` semantics)
"""

import os
from pathlib import Path


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
                if line and not line.startswith("#") and "=" in line:
                    key, _, val = line.partition("=")
                    os.environ.setdefault(key.strip(), val.strip().strip('"\''))
