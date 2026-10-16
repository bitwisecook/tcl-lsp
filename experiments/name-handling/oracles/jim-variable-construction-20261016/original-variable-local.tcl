set root before
variable root after
puts [list ROOT $root]
namespace eval P {
    variable marker first
    variable marker second
    puts [list NAMESPACE $marker]
    set status [catch {variable item(k) value} message]
    puts [list ELEMENT $status $message]
}
variable rootdict(k) value
puts [list ROOT_ELEMENT $rootdict(k)]
