if {[llength [info commands ::oo::class]] == 0} {
    puts {NOT_APPLICABLE TclOO unavailable}
    exit 0
}
proc helper {args} {return OLD}
oo::class create C {method m {} {::helper {[replaceHelper]}}}
set instance [C new]
puts [list before [$instance m]]
rename ::helper ::oldHelper
proc helper {args} {return NEW}
puts [list after [$instance m]]
