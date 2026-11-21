namespace eval ::lib { proc helper {} { return ::lib::helper } }
namespace eval ::app {
    proc run {} { helper ;# CALL
    }
    namespace path ::lib
}
puts [::app::run]
