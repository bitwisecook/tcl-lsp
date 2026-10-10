if {[info commands dict] eq ""} {return NOT_APPLICABLE}
set d {old BASE}
upvar 0 d linked
set code [catch {dict set linked added NEW} result]
list $code $result $d $linked
