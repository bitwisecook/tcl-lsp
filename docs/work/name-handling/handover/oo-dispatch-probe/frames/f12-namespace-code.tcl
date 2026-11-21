proc ::helper {} { return ::helper }
namespace eval ::lib {
    proc helper {} { return ::lib::helper }
    variable cb [namespace code { helper ;# CALL
    }]
}
puts [eval $::lib::cb]
