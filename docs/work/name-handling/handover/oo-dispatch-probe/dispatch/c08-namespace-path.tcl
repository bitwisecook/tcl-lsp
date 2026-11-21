namespace eval ::lib { proc helper {} { return helped } }
namespace eval ::app {
    namespace path ::lib
    proc run {} { return [helper] }
}
puts [::app::run]
