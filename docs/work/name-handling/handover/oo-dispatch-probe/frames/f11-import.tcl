proc ::helper {} { return ::helper }
namespace eval ::lib { namespace export helper; proc helper {} { return ::lib::helper } }
namespace eval ::app {
    namespace import ::lib::helper
    proc run {} { helper ;# CALL
    }
}
puts [::app::run]
