set code [catch {proc missing::audit {} {return NEVER}} message]
puts [list $code [namespace exists missing] [info commands audit]]
