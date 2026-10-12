package require tcltest
rename ::tcltest::test ::tcltest::original_test
proc ::tcltest::test {id description args} {
    array set options $args
    set body $options(-body)
    if {[llength $body] != 2 || [lindex $body 0] ne "isolated"} {error "unexpected isolation body"}
    set question [lindex $body 1]
    foreach value [list $id $description $question $options(-result)] {
        binary scan $value H* hex
        puts $hex
    }
}
rename ::tcltest::cleanupTests ::tcltest::original_cleanup
proc ::tcltest::cleanupTests {} {}
source [binary format H* 2f776f726b73706163652f74636c2d6c73702f6578706572696d656e74732f6e616d652d68616e646c696e672f7061747465726e732f77696b692d70726f6265732e74636c]
