#!/bin/bash
host=${1:-127.0.0.1}
port=${2:-5555}

# reading from stdin and send through nc, choosing keys for a specific netcat
if nc -h 2>&1 | grep -qi openbsd; then
  # OpenBSD/macOS: close socket after EOF on stdin -> -N
  nc -N "$host" "$port"
else
  # GNU: close through N seconds after EOF -> -q 1
  nc -w 2 -q 1 "$host" "$port"
fi