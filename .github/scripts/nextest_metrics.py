#!/usr/bin/env python3
"""Compute nextest wall time / test-CPU parallelism from a tee'd nextest log.

Used by the `rust-tests` job's "cargo nextest run" step (see
.github/workflows/ci.yml) to make a runner-host slowdown diagnosable
straight from the CI log, after the 2026-08-17/18 tank-host incident (see
that job's header comment for the full story). Prints one greppable
`nextest-metrics:` line, and an `::warning::` GitHub Actions annotation
when computed parallelism looks collapsed relative to the host's core
count.

Never raises: any input that doesn't match the expected nextest log shape
just prints a "skipping" note and exits 0, so a nextest output-format
change can never fail the job this is diagnosing.

Usage: nextest_metrics.py <log-path> <fallback-wall-seconds> <host-cores>
"""
import re
import sys


def main() -> int:
    if len(sys.argv) != 4:
        print("nextest-metrics: wrong arguments, skipping")
        return 0

    log_path, fallback_wall_arg, host_cores_arg = sys.argv[1:4]

    try:
        fallback_wall = float(fallback_wall_arg)
    except ValueError:
        fallback_wall = 0.0

    try:
        host_cores = float(host_cores_arg)
    except ValueError:
        host_cores = 0.0

    try:
        with open(log_path, "r", errors="replace") as f:
            text = f.read()
    except OSError as exc:
        print(f"nextest-metrics: could not read log ({exc}), skipping")
        return 0

    # e.g. "        PASS [   0.123s] crate::mod test_name"
    test_cpu = sum(
        float(m) for m in re.findall(r"PASS\s+\[\s*([0-9]+\.[0-9]+)s\]", text)
    )
    # e.g. "     Summary [  46.7s] 17034 tests run: 17034 passed, 0 skipped"
    summary = re.search(r"Summary\s+\[\s*([0-9]+\.[0-9]+)s\]", text)

    if summary is None or test_cpu <= 0:
        print(
            "nextest-metrics: log format not recognized "
            "(nextest output may have changed), skipping"
        )
        return 0

    wall = float(summary.group(1))
    if wall <= 0:
        wall = fallback_wall
    parallelism = (test_cpu / wall) if wall > 0 else 0.0
    cores_display = int(host_cores) if host_cores == int(host_cores) else host_cores
    print(
        f"nextest-metrics: wall={wall:.1f}s test_cpu={test_cpu:.1f}s "
        f"parallelism={parallelism:.2f} host_cores={cores_display}"
    )

    if host_cores > 0 and parallelism < (host_cores / 2):
        print(
            "::warning::rust-tests nextest parallelism is "
            f"{parallelism:.2f} (< half of {cores_display} host cores). "
            "Likely cause: the tank runner host (fewer usable cores / "
            "memory pressure during linking), not this commit's tests -- "
            "this exact signature occurred 2026-08-17. Check the tank box "
            "(see the Runner host telemetry step above) before bisecting "
            "commits."
        )

    return 0


if __name__ == "__main__":
    sys.exit(main())
