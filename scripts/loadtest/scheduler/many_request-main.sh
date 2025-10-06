#!/usr/bin/env bash
set -euo pipefail

# Parameters (change them via env at startup)
TOTAL_SUBMITS=${TOTAL_SUBMITS:-130}   # How many submits should I send in total
PING_EVERY=${PING_EVERY:-15}           # how many submits to send 1 PING (under your HIGH_BUDGET)
CONCURRENCY=${CONCURRENCY:-128}        # the limit of simultaneous processes

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

run_with_limit() {
  # Runs the command in the background, respecting the concurrency limit
  local cmd="$1"
  while (( $(jobs -rp | wc -l) >= CONCURRENCY )); do
    # a very short "yield" to avoid burning the CPU
    sleep 0.002
  done
  bash -c "$cmd" &
}

sent=0

# We're pouring submits in the background, periodically throwing in PING
while (( sent < TOTAL_SUBMITS )); do
  # increment first so indexing is 1-based
  (( ++sent ))
  # pass submit index to the child
  run_with_limit "SUBMIT_INDEX=$sent \"$SCRIPT_DIR/request_submit.sh\""

  # Every PING_EVERY submits an immediate PING, also in the background
  if (( sent % PING_EVERY == 0 )); then
    # pass the same overall index so you know at which submit the ping happened
    run_with_limit "PING_INDEX=$sent \"$SCRIPT_DIR/request_ping.sh\""
  fi
done

# Just in case, add a final PING if TOTAL_SUBMITS not a multiple of PING_EVERY
if (( sent % PING_EVERY != 0 )); then
  run_with_limit "PING_INDEX=$sent \"$SCRIPT_DIR/request_ping.sh\""
fi

# Wait for everyone
wait || true