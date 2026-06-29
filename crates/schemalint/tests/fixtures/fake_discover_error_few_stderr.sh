#!/bin/sh
# Fake JSON-RPC sidecar: reads one request from stdin, emits a few stderr lines
# (≤10 to exercise the "short stderr" arm in augment_error), then responds
# with a JSON-RPC DiscoverFailed error.
#
# Invoked as: fake_discover_error_few_stderr.sh <any args> (args are ignored)

# Emit exactly 3 stderr lines — well under the 10-line threshold.
echo "fake-sidecar: line one" >&2
echo "fake-sidecar: line two" >&2
echo "fake-sidecar: line three" >&2

# Drain the stdin request line.
read -r _request

# Emit the DiscoverFailed JSON-RPC error response.
printf '{"jsonrpc":"2.0","error":{"code":-32000,"message":"fake short-stderr discovery error"},"id":1}\n'
