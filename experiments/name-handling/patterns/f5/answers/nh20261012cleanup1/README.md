# Owned remote staging cleanup

Seventeen captured commands each ended with a successful exact `test ! -e` check for their owned `/var/tmp/tcl_lsp_<run-id>` path. The original sealed captures were retained locally before cleanup. Each command rejects a symlink at the staging path, lists regular files to depth two when a directory exists, removes that exact path, and checks its absence. All recorded command exits are 0 and all stderr files are empty.

The retained stdout inventories list 231 files across fifteen paths. The stock2 and stock3 inventories are empty; their successful after-checks establish absence without proving that either directory or any files existed before cleanup. Inventories are bounded by depth two and do not carry remote content hashes.

`receipts.json` retains the exact command argv, staging path and captured result for each run. `independent-review.json` hashes each original provenance, seal and cleanup transcript, verifies all 12,001 original sealed local files, and checks each staging path against its original run provenance. These receipts concern staging files at the captured cleanup time. Configuration objects and interpreter state have separate, scoped original-run cleanup receipts; no namespace, native procedure retirement or later state is inferred from staging deletion. No configuration was saved.
