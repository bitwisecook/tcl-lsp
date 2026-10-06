if {[catch {
    rename ::tcl::info::locals ::private
    rename ::info ::ensemble
    namespace ensemble configure ::ensemble -map {member ::private} -subcommands {member} -prefixes 1
} failure]} {
    puts [list surface unavailable $failure]
    exit 0
}
foreach {label source} {accepted {ensemble mem *} declined {ensemble member a b}} {
    proc p {} $source
    if {![catch {::tcl::unsupported::disassemble proc p} disassembly]} {
        puts [list disassembly $label $disassembly]
    }
    set code [catch p result]
    puts [list result $label $code $result]
    rename p {}
}
