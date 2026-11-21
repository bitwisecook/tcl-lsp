package require itcl
itcl::class C {
    method m {d} {
        binary scan $d c b
        if {$b == 0x81} {}
        set unused 1
        set m [expr {$d & 0xff == 1}]
        return $never
    }
}
