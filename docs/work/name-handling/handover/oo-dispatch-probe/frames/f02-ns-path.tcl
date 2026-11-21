proc ::helper {} { return ::helper }
namespace eval ::lib { proc helper {} { return ::lib::helper } }
namespace eval ::app {
    namespace path ::lib
    proc run {} { helper ;# CALL
    }
}
puts [::app::run]
