# The analyser must recurse into `catch { ... }` bodies, so the unbraced
# `expr` below is a W100 (expression not braced) just as it would be at the
# top level.
catch { set y [expr $a + $b] } msg
