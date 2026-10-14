# VM naming baseline

All four actual selected test targets build and45 exact controls execute:44 pass and one fails. All35 prior controls are retained, with ten added local/child/location/hash controls. Captured source/provider closure, actual Cargo dependency inputs and artifacts have zero drift. The selected original C8.4–9.1 libraries/init/tcltest inputs and pinned Jim interpreter pass their identity probes; a fresh read-only upstream check still matches that Jim revision.

Original local cleanup, source-location and child publication regressions pass. The unchanged20-program Jim automatic-error control fails at parse-procedure frame spelling: the VM reports fail for a parser frame where Jim reports an empty command. Original script/assertions remain required. This baseline predates allocator pooling integration. Exact binaries, selectors, provider/input hashes and raw outputs are retained in result.json.
