puts "PATCHLEVEL|[info patchlevel]"
foreach key {library nameofexecutable loaded} {set code [catch [list info $key] value]; puts "INFO|$key|$code|$value"}
set code [catch {encoding system} value]; puts "ENCODING|$code|$value"
