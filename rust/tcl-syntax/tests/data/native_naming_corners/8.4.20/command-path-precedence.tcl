set code [catch {namespace eval ::n {}; namespace eval ::path {proc leaf {} {return PATH}}; proc ::leaf {} {return ROOT}; namespace eval ::n {namespace path ::path; leaf}} result]
binary scan $result H* hex
puts [list $code $hex]
