"""JSON-RPC 2.0 server over stdin/stdout.

Reads one JSON-RPC request per line from stdin, dispatches to the
appropriate handler, and writes a single-line JSON response to stdout.
"""

import json
import sys
import traceback

from schemalint_pydantic.discover import discover_models

_METHODS = {"discover", "shutdown"}


def main() -> None:
    """Run the server loop."""
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
        except json.JSONDecodeError:
            _send_error(None, -32700, "Parse error")
            continue

        if request.get("jsonrpc") != "2.0":
            _send_error(request.get("id"), -32600, "Invalid JSON-RPC: missing or incorrect jsonrpc field")
            continue

        method = request.get("method", "")
        req_id = request.get("id")

        if method == "discover":
            _handle_discover(request, req_id)
        elif method == "shutdown":
            _send_response(req_id, "ok")
            break
        elif method == "":
            _send_error(req_id, -32600, "Invalid JSON-RPC request: missing method")
        else:
            _send_error(req_id, -32601, f"Method not found: {method}")


def _handle_discover(request: dict, req_id) -> None:
    params = request.get("params", {})
    package = params.get("package")
    if not package or not isinstance(package, str):
        _send_error(req_id, -32602, "Missing or invalid 'package' parameter")
        return

    try:
        result = discover_models(package)
        _send_response(req_id, result)
    except Exception as e:
        tb = traceback.format_exc()
        _send_error(req_id, -32603, f"Discovery failed: {e}", data={"traceback": tb})


def _send_response(req_id, result) -> None:
    response = {"jsonrpc": "2.0", "result": result, "id": req_id}
    sys.stdout.write(json.dumps(response, default=str) + "\n")
    sys.stdout.flush()


def _send_error(req_id, code: int, message: str, data=None) -> None:
    error = {"code": code, "message": message}
    if data is not None:
        error["data"] = data
    response = {"jsonrpc": "2.0", "error": error, "id": req_id}
    sys.stdout.write(json.dumps(response, default=str) + "\n")
    sys.stdout.flush()
