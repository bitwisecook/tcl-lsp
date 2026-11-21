namespace eval ::lib { namespace export x; proc x {} {return lib} }
namespace eval ::app { proc x {} {return app}; puts [catch {namespace import ::lib::x} m]:$m; namespace import -force ::lib::x; puts [x] }
