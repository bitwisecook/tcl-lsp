# Issue #923 differential audit (namespaces / packages closeout): placeholder
# fixture. `issue923NsPkgAudit.test.ts` replaces this content at runtime (via
# setTestContent) with each specific repro snippet -- checking in one file per
# tiny snippet would bloat the repo for no benefit. The one case that genuinely
# needs a second document on disk (a cross-file namespace variable) uses the
# committed `issue923NsPkgAuditLib.tcl` beside this file.
proc issue923NsPkgAuditPlaceholder {} {
    return 1
}
