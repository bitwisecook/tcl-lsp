# Final owned staging and configuration checks

Six original capture bundles were retrieved and locally seal-verified before exact owned staging cleanup. Their original provenance and seal hashes are retained in `receipts.json`. Independent review verified all 13,945 original sealed files without changing those bundles.

The initial attempt retained inventories totalling 328 regular files across the six exact `/var/tmp/tcl_lsp_<run-id>` directories. Its helper asserts each staging command's result 0 before proceeding to configuration listings. The separate retained `ltm-rule.stdout` then reports `listing-exit=1`; stderr identifies `one-line` as a missing iRule. Thus staging cleanup had completed before that listing failure. There is no successful configuration-absence result from this initial listing.

The first retry repeated the staging checks with empty inventories, but its intended listing syntax change was not applied. The retained helper still uses `one-line`, and `ltm-rule.retry.stdout`/stderr retain the same listing failure.

The final retry removed `one-line` from the exact tmsh list commands. `receipts.json` records this final attempt: all six staging commands returned 0 with empty inventories and empty stderr, confirming each exact staging path remained absent. These later empty inventories are absence checks; they are not the original file-removal inventory. Every command rejects a staging-path symlink, optionally lists regular files to depth two, removes the exact owned path and finishes with `test ! -e` for that path. Remote file content hashes and a complete recursive inventory were not measured.

All ten final listing commands returned 0. Each stdout contains only `listing-exit=0`, and each stderr is empty. The remote filter retained lines matching `__tcl_lsp_probe_nh20261012`; there are no matching lines in these actual listing scopes:

| Successful exact command type | Matching task-prefix lines |
| --- | --- |
| `ltm rule` | 0 |
| `ltm virtual` | 0 |
| `ltm pool` | 0 |
| `sys application template` | 0 |
| `sys application service` | 0 |
| `cli script` | 0 |
| `sys icall script` | 0 |
| `sys icall handler periodic` | 0 |
| `sys folder` | 0 |
| `auth partition` | 0 |

The full unfiltered appliance listings are not retained. The receipts support absence of the task prefix in these ten exact command outputs at the captured time. They do not establish broader interpreter state or retirement of native procedure catalogues. Runtime cleanup and the remaining original-name limitations are described by each capture and the separate hosted cleanup review. No configuration was saved.

`independent-review.json` anchors the original seals, all three helper snapshots, initial and retry transcripts, and successful final filtered listings. No semantic proof is derived from this cleanup bundle.
