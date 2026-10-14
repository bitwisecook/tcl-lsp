# Compiler write-effect fixture build

No semantic controls execute. Compilation rejects a new custom write-effect fixture because CommandRegistry has no Clone implementation and cloning its reference does not construct an owned store. Captured source subset drift is zero. Use a genuine current registry constructor while preserving source/input/custom descriptor identity and all original assertions.
