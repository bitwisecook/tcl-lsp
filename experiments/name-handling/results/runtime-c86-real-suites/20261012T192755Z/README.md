# Frame-enabled wrapper attempt

This retained attempt copied the older C8.4 runner's `interp debug {} -frame 1` prelude. The real C8.6 interpreter suite expects the default frame value 0 at `interp-38.4` and `interp-38.5`; these two control failures are attributable to the wrapper mutation. No upstream source was edited.

All 18 processes completed without timeout, crash or OOM. Actual original sources, wrappers, hashes and streams remain preserved. Final evidence uses the fresh upstream-consistent in-process attempt at [20261012T193343Z](../20261012T193343Z/README.md); this attempt supplies no aggregate default-frame verdict.
