puts [list version [info patchlevel]]
set code [catch {package require tcltest} version]
puts [list package $code $version]
if {$code == 0} {puts [list package_ifneeded [package ifneeded tcltest $version]]; puts [list auto_path $auto_path]}
