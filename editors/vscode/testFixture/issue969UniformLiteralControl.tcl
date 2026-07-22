# Issue #969 TP control: the interprocedural call-site-literal seed must
# still fire I230 when a parameter genuinely IS invariant across every
# caller — two callers passing the identical literal to a private helper.
proc helper {mode} {
    if {$mode eq "prod"} {
        set r 1
    } else {
        set r 2
    }
}
proc caller1 {} {
    helper prod
}
proc caller2 {} {
    helper prod
}
