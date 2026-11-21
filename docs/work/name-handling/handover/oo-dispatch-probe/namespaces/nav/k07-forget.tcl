namespace eval ::lib { namespace export f; proc f {} {} }
namespace eval ::app { namespace import ::lib::f; namespace forget ::lib::f; puts [catch {f} m]:$m ;# CALL f
}
