"""Tests for the JSON-RPC server."""

import json
import importlib
import subprocess
import sys
import os
import tempfile
import textwrap
import time
import uuid

import pytest


@pytest.fixture
def temp_package():
    """Create a temporary Python package with a Pydantic v2 model."""
    pkg_name = f"testmodels_{uuid.uuid4().hex[:8]}"
    with tempfile.TemporaryDirectory() as tmpdir:
        pkg_dir = os.path.join(tmpdir, pkg_name)
        os.makedirs(pkg_dir)
        init_path = os.path.join(pkg_dir, "__init__.py")

        init_content = textwrap.dedent("""
        from pydantic import BaseModel

        class HelloModel(BaseModel):
            greeting: str
            target: str
        """)

        with open(init_path, "w") as f:
            f.write(init_content)

        try:
            yield tmpdir, pkg_name
        finally:
            sys.modules.pop(pkg_name, None)
            importlib.invalidate_caches()


def _start_server(extra_paths=None):
    """Start the schemalint-pydantic server as a subprocess."""
    env = os.environ.copy()
    py_path = (
        os.path.join(os.path.dirname(__file__), "..", "src")
        + os.pathsep
        + env.get("PYTHONPATH", "")
    )
    if extra_paths:
        py_path = extra_paths + os.pathsep + py_path
    env["PYTHONPATH"] = py_path
    proc = subprocess.Popen(
        [sys.executable, "-m", "schemalint_pydantic"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )
    return proc


def _send_request(proc, request: dict) -> dict:
    """Send a JSON-RPC request and read the response."""
    req_line = json.dumps(request) + "\n"
    proc.stdin.write(req_line)
    proc.stdin.flush()
    resp_line = proc.stdout.readline()
    if not resp_line:
        raise RuntimeError("No response received")
    return json.loads(resp_line)


class TestServerProtocol:
    """JSON-RPC protocol tests."""

    def test_discover_roundtrip(self, temp_package):
        tmpdir, pkg_name = temp_package
        try:
            import pydantic  # noqa: F401
        except ImportError:
            pytest.skip("pydantic not installed")

        proc = _start_server(extra_paths=tmpdir)
        try:
            request = {
                "jsonrpc": "2.0",
                "method": "discover",
                "params": {"package": pkg_name},
                "id": 1,
            }
            response = _send_request(proc, request)
        finally:
            proc.stdin.close()
            proc.wait(timeout=5)

        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 1
        assert "result" in response
        result = response["result"]
        assert "models" in result
        models = result["models"]
        assert len(models) == 1
        assert models[0]["name"] == "HelloModel"
        assert "schema" in models[0]
        assert "source_map" in models[0]

    def test_shutdown(self):
        proc = _start_server()
        request = {
            "jsonrpc": "2.0",
            "method": "shutdown",
            "id": 1,
        }
        response = _send_request(proc, request)
        assert response["jsonrpc"] == "2.0"
        assert response["result"] == "ok"

        # Process should exit after shutdown
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()

    def test_invalid_json(self):
        proc = _start_server()
        try:
            proc.stdin.write("not valid json\n")
            proc.stdin.flush()
            resp_line = proc.stdout.readline()
            response = json.loads(resp_line)
        finally:
            proc.stdin.close()
            proc.wait(timeout=5)

        assert response["jsonrpc"] == "2.0"
        assert "error" in response
        assert response["error"]["code"] == -32700

    def test_unknown_method(self):
        proc = _start_server()
        try:
            request = {
                "jsonrpc": "2.0",
                "method": "nonexistent",
                "id": 1,
            }
            response = _send_request(proc, request)
        finally:
            proc.stdin.close()
            proc.wait(timeout=5)

        assert response["jsonrpc"] == "2.0"
        assert "error" in response
        assert response["error"]["code"] == -32601

    def test_missing_method(self):
        proc = _start_server()
        try:
            request = {
                "jsonrpc": "2.0",
                "id": 1,
            }
            response = _send_request(proc, request)
        finally:
            proc.stdin.close()
            proc.wait(timeout=5)

        assert response["jsonrpc"] == "2.0"
        assert "error" in response
        assert response["error"]["code"] == -32600

    def test_missing_jsonrpc_field(self):
        proc = _start_server()
        try:
            request = {
                "method": "shutdown",
                "id": 1,
            }
            response = _send_request(proc, request)
        finally:
            proc.stdin.close()
            proc.wait(timeout=5)

        assert "jsonrpc" in response
        assert "error" in response
        assert response["error"]["code"] == -32600

    def test_discover_missing_package(self):
        proc = _start_server()
        try:
            request = {
                "jsonrpc": "2.0",
                "method": "discover",
                "params": {},
                "id": 1,
            }
            response = _send_request(proc, request)
        finally:
            proc.stdin.close()
            proc.wait(timeout=5)

        assert "error" in response
        assert response["error"]["code"] == -32602

    def test_discover_invalid_package(self):
        proc = _start_server()
        try:
            request = {
                "jsonrpc": "2.0",
                "method": "discover",
                "params": {"package": "nonexistent_pkg_12345"},
                "id": 1,
            }
            response = _send_request(proc, request)
        finally:
            proc.stdin.close()
            proc.wait(timeout=5)

        assert "error" in response
        assert response["error"]["code"] == -32603
