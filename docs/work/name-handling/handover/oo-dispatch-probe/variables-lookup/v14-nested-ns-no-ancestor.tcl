namespace eval ::a { variable v outer }
namespace eval ::a::b { puts [catch {set w $v} m]:$m ;# READ
}
