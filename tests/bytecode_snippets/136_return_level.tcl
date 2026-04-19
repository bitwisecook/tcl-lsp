proc inner {} {
    return -code error -level 0 "local error"
}
proc outer {} {
    set code [catch {inner} msg]
    list $code $msg
}
