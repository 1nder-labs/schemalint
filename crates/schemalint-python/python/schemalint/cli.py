"""Python console entry point for the packaged schemalint binary."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys


def main() -> None:
    """Delegate to the Rust binary bundled by maturin."""
    binary = shutil.which("schemalint-python-bin")
    if binary is None:
        raise SystemExit("schemalint binary not found in this Python environment")

    args = [binary, *sys.argv[1:]]
    if os.name == "nt":
        raise SystemExit(subprocess.call(args))

    os.execv(binary, args)
