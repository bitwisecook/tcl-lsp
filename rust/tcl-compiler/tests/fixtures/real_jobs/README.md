# Real-job Tcl compiler fixtures

These are verbatim, commit-pinned source files from independent projects. They
are vendored so compiler tests never use the network. No deterministic runtime
output oracle is asserted: neither upstream file publishes standalone input /
output vectors for the complete script.

## Sqawk Assemble

- Fixture: `sqawk/assemble.tcl`
- Upstream project: <https://github.com/dbohdan/sqawk>
- Upstream path: `tools/assemble.tcl`
- Commit: `cc5af0b87c31c0dde3278bc05af9fb1a6af1bed9`
- Source: <https://github.com/dbohdan/sqawk/blob/cc5af0b87c31c0dde3278bc05af9fb1a6af1bed9/tools/assemble.tcl>
- Copyright: 2015–2019, 2024 D. Bohdan
- SPDX-License-Identifier: MIT
- Complete licence: `sqawk/LICENSE` (verbatim from the same commit)
- Local modifications: none

## OdoCrypt FPGA miner checksum

- Fixture: `odo-miner/checksum.tcl`
- Upstream project: <https://github.com/MentalCollatz/odo-miner>
- Upstream path: `src/miner/checksum.tcl`
- Commit: `d4c4ba9609d228b9cb9d1c2f29a80ebcd367b155`
- Source: <https://github.com/MentalCollatz/odo-miner/blob/d4c4ba9609d228b9cb9d1c2f29a80ebcd367b155/src/miner/checksum.tcl>
- Copyright: 2019 MentalCollatz
- SPDX-License-Identifier: GPL-3.0-or-later
- Complete licence: `odo-miner/LICENSE` (verbatim from the same commit; the
  source's licence notice grants GPL version 3 or any later version)
- Local modifications: none
