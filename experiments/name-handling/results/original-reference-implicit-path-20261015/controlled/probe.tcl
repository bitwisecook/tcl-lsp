if {[llength [info commands ::oo::class]] == 0} {
    puts {TCLOO unavailable}
} else {
    proc ::oo::Helpers::sharedHelper {} {return HELPERS}
    proc ::oo::sharedDirect {} {return OO}
    ::oo::class create NamingProbe {
        method examine {} {list [namespace path] [catch {sharedHelper} a] $a [catch {sharedDirect} b] $b}
    }
    NamingProbe create namingProbe
    puts [namingProbe examine]
    puts [list [catch {sharedHelper} a] $a [catch {sharedDirect} b] $b]
}
