
        namespace eval ::N {
            variable value ready
            proc get {} {variable value; return $value}
            proc replace {} {variable value changed; return $value}
        }
        list [::N::get] [::N::replace] [::N::get]
    