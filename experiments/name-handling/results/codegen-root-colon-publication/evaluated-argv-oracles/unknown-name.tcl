set code [catch {proc $unknown_name {} {return NEVER}} message]
puts [list $code $message [info commands :audit]]
