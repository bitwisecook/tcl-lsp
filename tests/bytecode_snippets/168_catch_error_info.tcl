proc test {} {
    if {[catch {expr {1/0}} result opts]} {
        return [list $result [dict get $opts -errorcode]]
    }
    return "ok"
}
