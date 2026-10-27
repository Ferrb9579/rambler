#!/usr/bin/env bash
# Keep Cargo, linkers, model workers, and their descendants in one cgroup.
set -uo pipefail

if [[ ${1:-} != --in-scope ]]; then
    if ! command -v systemd-run >/dev/null || ! command -v timeout >/dev/null; then
        echo "Constrained tests require Linux, a systemd user session, and GNU timeout." >&2
        exit 1
    fi
    exec systemd-run --user --scope --quiet \
        -p MemoryAccounting=yes \
        -p "MemoryHigh=${RAMBLER_TEST_MEMORY_HIGH:-6G}" \
        -p "MemoryMax=${RAMBLER_TEST_MEMORY_MAX:-8G}" \
        -p MemorySwapMax=0 \
        -p OOMPolicy=kill \
        "$(realpath "$0")" --in-scope "$@"
fi
shift

cd "$(dirname "$(realpath "$0")")/.." || exit 1
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
# Model tests must remain serial even if the caller's environment says otherwise.
export RUST_TEST_THREADS=1
if (( $# == 0 )); then
    set -- cargo test --all-targets --locked
fi

read -r cgroup_line < /proc/self/cgroup
cgroup="/sys/fs/cgroup${cgroup_line#0::}"
if [[ ! -r $cgroup/memory.max || $(cat "$cgroup/memory.max") == max \
    || ! -r $cgroup/memory.swap.max || $(cat "$cgroup/memory.swap.max") != 0 ]]; then
    echo "The requested RAM/swap limits are not enforced; refusing to run." >&2
    exit 1
fi
echo "Memory limit: $(cat "$cgroup/memory.max") bytes; swap disabled; timeout: ${RAMBLER_TEST_TIMEOUT:-20m}" >&2
timeout --signal=TERM --kill-after=10s "${RAMBLER_TEST_TIMEOUT:-20m}" "$@"
status=$?
if [[ -r $cgroup/memory.peak ]]; then
    echo "Peak cgroup memory: $(cat "$cgroup/memory.peak") bytes (includes children and file cache)" >&2
fi
if (( status == 124 || status == 137 )); then
    echo "Command timed out or hit a resource limit (exit $status)." >&2
fi
exit "$status"
