namespace eval ::e { namespace export only; proc only {} { return only }; namespace ensemble create }
puts [catch {e ""} m]:$m
