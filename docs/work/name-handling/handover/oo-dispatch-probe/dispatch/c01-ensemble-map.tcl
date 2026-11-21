namespace eval ::geo {
    proc area {w h} { expr {$w * $h} }
    namespace ensemble create -map {area ::geo::area}
}
puts [geo area 2 3]
puts [catch {geo area 2} m]:$m ;# BAD arity
puts [catch {geo volume 1} m]:$m ;# BAD sub
