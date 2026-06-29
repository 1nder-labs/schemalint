#!/bin/sh
# Fake JSON-RPC sidecar: reads one request from stdin, writes some stderr lines,
# then responds with a JSON-RPC error (DiscoverFailed).
# Used by tests that exercise the augment_error + DiscoverFailed branch.
#
# Invoked as: fake_discover_error.sh <any args>
# (The node helper calls: node-path <helper-script>, so $1 is the helper path
#  and we just ignore it. The python helper calls: python-path -m schemalint_pydantic,
#  so $1=-m $2=schemalint_pydantic; we ignore those too.)

# Emit several stderr lines so augment_error gets stderr content to attach.
echo "fake-sidecar: starting up" >&2
echo "fake-sidecar: reading request" >&2
echo "fake-sidecar: preparing error response" >&2

# Read one line from stdin (the discover request) — must drain it so the
# write-side of the Rust pipe doesn't block/fail before we respond.
read -r _request

# Emit the DiscoverFailed JSON-RPC error response on stdout.
printf '{"jsonrpc":"2.0","error":{"code":-32000,"message":"fake discovery failure: module not found"},"id":1}\n'
