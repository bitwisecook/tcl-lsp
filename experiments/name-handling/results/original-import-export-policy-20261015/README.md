# Original import export-filter and self-import observations

This original shell capture compares C Tcl8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and the pinned Jim provider. Each actual provider runs the same unchanged probe; no tcltest is used. All six process exits are0. The manifest records binary and init.tcl or nshelper.tcl hashes before and after, with no changes. Jim freshness remains the separate timed04:25 upstream receipt from the [Origin SDK archive](../original-jim-namespace-origin-sdk-20261015/README.md); this capture makes no continuous latest claim.

Without namespace export, the import returns0 on all six. C's target call returns1 because no command was imported; Jim's target call returns0 with SOURCE. All six reject importing a namespace into itself, and the original local command remains callable as SELF. These exact four output rows per provider establish only this branch of the original import helper. Existing [escaped import observations](../original-workspace-import-patterns-20261015/manifest.json) independently cover the four escaped pattern positives on all six providers.

This archive grants no source census, alias-chain or conflict/replacement completion, Runtime execution, native frame identity or full lifecycle conformance. Runtime and static source-advice implementations must retain their separate command presence, policy, conflict, alias lifecycle and unknown-census guards. Original absolute paths in the capture driver/manifest are provenance; verification uses relative retained files. No binaries or full SDK build closure are archived.

Run `python3 verify.py` to verify retained file hashes and unchanged manifest provider hashes. `python3 verify.py --current-provider` additionally hashes the recorded local provider files without running them.
