# Original scalar variable-name mutations

The direct original C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and retained Jim 0.84-9-g5bac7c9 interpreters increment initialized empty- and dollar-named scalars from 8 to 9. C8.4 rejects an increment of the absent empty variable; later C releases and this Jim initialise it to 1. An initialized ordinary counter changes from 9 to 10. C read/write trace callbacks occur twice for one increment. This Jim has no trace command; both attempted APIs remain explicit failures. C9 rejects the legacy trace syntax; its accepted modern syntax is retained separately.

Both original programs, raw stdout/stderr, original executable and release-library hashes, matching original init.tcl files where present, and unchanged-input receipts are retained. No tcltest driver was used. These direct interpreter observations do not establish compiler behaviour, Native handler/store/frame ownership, F5 results or complete host/loader dependency closure.

`capture.py --repo /path/to/tcl-lsp --out /tmp/fresh-scalar-oracles` reruns the retained modern/fallback program against original interpreter trees; the output directory must be new. `--probe legacy` selects the independently retained legacy program. Original capture scripts retain the exact source paths used by the recorded observations.
