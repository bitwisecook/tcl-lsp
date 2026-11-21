namespace eval ::geo { namespace export area; proc area {w h} { expr {$w*$h} }; proc vol {} {}; namespace ensemble create -subcommands {area} }
puts [list [geo area 2 3] [catch {geo vol} m] $m [namespace ensemble exists ::geo] [dict get [namespace ensemble configure ::geo] -subcommands]]
