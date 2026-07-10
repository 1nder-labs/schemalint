import os
import zipfile

import pytest


def test_built_wheel_contains_public_command_and_bundled_sidecar():
    wheel = os.environ.get("SCHEMALINT_WHEEL")
    if wheel is None:
        pytest.skip("set SCHEMALINT_WHEEL to run the built-artifact contract")

    with zipfile.ZipFile(wheel) as archive:
        names = set(archive.namelist())

    assert any(name.endswith(".data/scripts/schemalint") for name in names)
    assert not any(name.endswith(".data/scripts/schemalint-python-bin") for name in names)
    assert "schemalint_pydantic/__main__.py" in names
    assert "schemalint_pydantic/discover.py" in names
    assert "schemalint_pydantic/server.py" in names
