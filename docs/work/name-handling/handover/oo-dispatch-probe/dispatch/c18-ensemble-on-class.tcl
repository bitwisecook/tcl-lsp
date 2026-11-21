oo::class create C { method m {} { return m } }
namespace eval ::e { namespace ensemble create -map {mk {C new}} }
puts [catch {[e mk] m} m]:$m ;# BAD relative map target
