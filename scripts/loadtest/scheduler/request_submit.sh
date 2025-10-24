#!/bin/bash
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# Use provided SUBMIT_INDEX or fallback to '?'
IDX=${SUBMIT_INDEX:-?}

printf "mining.submit\n" | "$SCRIPT_DIR/nc_send.sh" 127.0.0.1 5555 | sed -e "s/^/[SUBMIT #$IDX] /"
