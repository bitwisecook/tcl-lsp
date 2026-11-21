proc ::helper {} { set ::r ::helper }
namespace eval ::a {
    proc helper {} { set ::r ::a::helper }
    proc run {} { after 0 { helper ;# CALL
    } }
}
::a::run
after 20 {set ::done 1}; vwait ::done
puts $::r
