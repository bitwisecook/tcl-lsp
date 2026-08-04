package provide example::meter 2.0

# Minimal TclOO module mirroring the file shape from issue #1122 (sticky
# scroll dead once the extension is enabled): a versioned Tcl module whose
# class definer's folding/outline ranges must stay inside the document,
# with real code trailing the class body. Identifiers here are invented,
# not the reporter's.
oo::class create ::example::meter::Meter {
    superclass ::oo::object

    variable Handle Config

    constructor {handle config} {
        set Handle $handle
        set Config $config
    }

    method open {} {
        set Handle [dict get $Config device]
        return $Handle
    }

    method close {} {
        set Handle {}
    }
}

proc ::example::meter::create {handle config} {
    return [::example::meter::Meter new $handle $config]
}
