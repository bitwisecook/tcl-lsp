proc forgetXyce {} {
    # Forgets all '::SpiceGenTcl::Xyce' commands from caller namespace
    uplevel 1 {foreach nameSpc [namespace children ::SpiceGenTcl::Xyce] {
        namespace forget ${nameSpc}::*
    }}
}

proc noLevelBody {} {
    uplevel {set counter 0}
}

proc globalBody {} {
    uplevel #0 {puts started}
}
