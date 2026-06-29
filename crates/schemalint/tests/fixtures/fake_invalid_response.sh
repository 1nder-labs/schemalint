#!/bin/sh
# Fake JSON-RPC sidecar: reads one request from stdin, emits many stderr lines
# (>10 to exercise the truncated stderr arm in augment_error), then responds
# with an invalid (non-JSON) line that triggers InvalidResponse parse error.
#
# Invoked as: fake_invalid_response.sh <any args> (args are ignored)

# Emit 12 stderr lines so the ">10 lines → truncated tail" branch is exercised.
for i in 1 2 3 4 5 6 7 8 9 10 11 12; do
    echo "fake-sidecar: stderr line $i" >&2
done

# Drain the stdin request line.
read -r _request

# Emit unparseable output — this causes an InvalidResponse error in send_discover.
printf 'this is definitely not valid json\n'
