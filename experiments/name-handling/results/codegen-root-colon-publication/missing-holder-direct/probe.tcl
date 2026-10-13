set code [catch {proc missing::audit {} {return CREATED}} message]
puts [list $code $message [namespace which -command missing::audit]]
