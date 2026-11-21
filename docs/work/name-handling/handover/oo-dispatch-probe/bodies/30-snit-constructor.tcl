package require snit
snit::type T {
    constructor {d} {
        binary scan $d c b
        if {$b == 0x81} {}
        set unused 1
        set m [expr {$d & 0xff == 1}]
        return $never
    }
}
