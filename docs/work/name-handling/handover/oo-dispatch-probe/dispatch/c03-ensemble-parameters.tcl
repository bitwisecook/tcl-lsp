namespace eval ::p {
    proc show {who what} { return $who:$what }
    namespace ensemble create -parameters who -map {show ::p::show}
}
puts [p me show it]
puts [catch {p me show} m]:$m ;# BAD
