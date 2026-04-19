proc choose_pool {host} { switch -- [string tolower $host] { api.example.com {return api_pool}   www.example.com { return web_pool } default {return default_pool} } }

proc normalise_user {first last env_name} {
set full [string trim "$first $last"]
switch -- $env_name { dev {set level debug}
test { set level info } prod {set level warn}
default
{set level info}}
if {[string length $full] > 40} {set full [string range $full 0 39]}
return [dict create user $full level $level]
}

set users [list Ada Lovelace dev Grace Hopper prod]
set report {}
foreach {first last env_name} $users {lappend report [normalise_user $first $last $env_name]}
puts $report
#   ^--- cursor
