#!/bin/bash
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# Use provided PING_INDEX or fallback to '?'
IDX=${PING_INDEX:-?}
PING_LINE="${PING_LINE:-PING}"
printf "%s\n" "$PING_LINE" | "$SCRIPT_DIR/nc_send.sh" 127.0.0.1 8080 | sed -e "s/^/[ PING #$IDX ] /"
