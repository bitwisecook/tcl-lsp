# Fresh reference interpreter survey used by cargo xtask release-facts.
puts "patchlevel\t[info patchlevel]"
foreach name [lsort [info functions]] { puts "math\t$name" }
foreach name [lsort [encoding names]] { puts "encoding\t$name" }
proc ::tcl_lsp_release_fact_commands {ns} {
    foreach name [info commands ${ns}::*] {
        if {$name ne "::tcl_lsp_release_fact_commands"} { puts "command\t$name" }
    }
    foreach child [namespace children $ns] { ::tcl_lsp_release_fact_commands $child }
}
::tcl_lsp_release_fact_commands {}
foreach name [lsort [package names]] {
    set version [package provide $name]
    if {$version ne ""} {
        puts "provided\t$name\t$version\t[expr {[llength [package versions $name]] > 0}]"
    }
}
if {[info exists ::oo::version]} {
    puts "oo\t$::oo::version\t$::oo::patchlevel"
}
