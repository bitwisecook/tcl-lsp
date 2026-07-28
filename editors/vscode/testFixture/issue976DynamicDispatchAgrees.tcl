# TP control for issue #976: the dispatch is resolved *by value* and
# recorded as an ordinary call site, so a dispatch that passes the same
# literal every other caller does must still fold — the fix must not
# blanket-disqualify the module the way the reverted first attempt did.
proc helper {mode} {
    if {$mode eq "prod"} {
        set x 1
    } else {
        set x 2
    }
}
helper prod
helper prod
set cmd helper
$cmd prod
