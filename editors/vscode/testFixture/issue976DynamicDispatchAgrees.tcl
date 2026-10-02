# TP control: the dispatch is resolved *by value* and
# recorded as an ordinary call site, so a dispatch that passes the same
# literal every other caller does must still fold — the module must not
# be blanket-disqualified just because it also contains dynamic dispatch.
proc i976agreeHelper {mode} {
    if {$mode eq "prod"} {
        set x 1
    } else {
        set x 2
    }
}
i976agreeHelper prod
i976agreeHelper prod
set cmd i976agreeHelper
$cmd prod
