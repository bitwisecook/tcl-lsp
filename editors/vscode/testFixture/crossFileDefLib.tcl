# Issue #1331 — the library half. The caller lives in crossFileCaller.tcl and
# does not `source` this file: the two are joined only by sharing a workspace,
# which is the shape every real multi-file Tcl project has.
proc libtest {a b c} {
    return [expr {$a + $b + $c}]
}
