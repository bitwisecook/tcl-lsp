namespace eval ::geo {
    namespace export area
    proc area {w h} { expr {$w * $h} }
    proc hidden {} {}
    namespace ensemble create
}
puts [geo ar 2 3]
puts [catch {geo hidden} m]:$m ;# BAD not exported
