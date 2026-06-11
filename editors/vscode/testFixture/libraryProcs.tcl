# FP-STY-13 fixture: redefining overridable Tcl *library* procedures.
# These are script-defined, user-replaceable library procs (not C built-ins),
# so redefining them must NOT fire W113.
proc unknown args { return }
proc history {args} { return }
proc auto_execok name { return }
# These ARE genuine built-in commands — redefining them MUST still fire W113.
proc set {a b} { return }
proc clock {a} { return }
