# Original #2286 phase observation

The unchanged original compiler control ran for 30 seconds against the retained Compiler92 executable. Its hash and original probe hash matched the compiled receipt; current source and executable drift were zero. The run timed out before the original assertion completed, so it provides no semantic acceptance.

Two analysis/CFG diagnostic cycles completed. The next cycle was preparing deferred source arguments at the cutoff. Inclusive timings overlap and must not be added. See `phase-summary.json`, `run.log`, `result.json` and the preserved runner. The source snapshots are stored as gzip-compressed JSON.
