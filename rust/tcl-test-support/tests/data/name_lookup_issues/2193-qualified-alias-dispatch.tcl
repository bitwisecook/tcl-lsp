interp alias {} ::fresh::alias {} list
puts [list [namespace exists ::fresh] [namespace which -command ::fresh::alias] [catch {::fresh::alias value} result] $result]
