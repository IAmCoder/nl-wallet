#!/bin/bash

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
NUM="${1:-1}"

cargo build --manifest-path "${SCRIPTS_DIR}/Cargo.toml" --features local

pids=()
for ((i=1; i <= NUM; i++)); do
  ("${SCRIPTS_DIR}"/../../target/debug/tests_integration_performance 2>&1) & pids+=($!)
done

# wait and collect return codes
rets=()
for pid in ${pids[*]}; do
    wait "$pid"
    rets+=($?)
done
echo "Return codes: ${rets[*]}"

error=false
for pid in ${pids[*]}; do
    if ! wait "$pid"; then
        error=true
    fi
done

if $error; then exit 1; fi
exit 0
