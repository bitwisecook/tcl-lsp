# ``catch {error MSG} var`` must put MSG into *var*.  Previously
# ``catch_leave`` cleared ``error_flag`` before ``catch_result`` ran,
# so ``catch_result`` always fell through to the success branch and
# returned the empty ``catch_ok_result``.  ``msg`` always arrived empty.
set rc [catch {error boom} msg]
if {$rc != 1} { error "rc = $rc (want 1)" }
if {$msg ne "boom"} { error "msg = '$msg' (want 'boom')" }

# Two catches in a row — the second must not leak state from the first.
set rc2 [catch {error zap} m2]
if {$m2 ne "zap"} { error "second catch leaked: $m2" }

# Catch without a result variable still observes the error.
set rc3 [catch {error noVarCase}]
if {$rc3 != 1} { error "catch-without-var rc = $rc3" }
return ok
