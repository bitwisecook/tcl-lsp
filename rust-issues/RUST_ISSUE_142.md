# RUST_ISSUE_142: `ensure_node` installs the distro `nodejs` (Ubuntu 24.04 apt ships Node 18.x), violating the project's documented Node 24+ minimum (Makefile:14, AGENTS.md:110, README.md:2067, CI `node-version: "24"`)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `scripts/dev/ensure-test-deps.sh:427-437` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

scripts/dev/ensure-test-deps.sh:427-437 — `ensure_node` installs the distro `nodejs` (Ubuntu 24.04 apt ships Node 18.x), violating the project's documented Node 24+ minimum (Makefile:14, AGENTS.md:110, README.md:2067, CI `node-version: "24"`).
`apt-get) run_install "Node.js (apt)" nodejs npm ;;` with no version check — a "successful" ensure-test-deps run leaves a toolchain two majors below the floor. Confidence: high
