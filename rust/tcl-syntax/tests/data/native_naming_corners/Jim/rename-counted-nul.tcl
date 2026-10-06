set code [catch {proc leaf {} {return ORIGINAL}; set dest "moved[format %c 0]tail"; rename leaf $dest; eval [list $dest]} result]
binary scan $result H* hex
puts [list $code $hex]
