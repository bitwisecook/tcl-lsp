set root before
variable root after
set nsresult [namespace eval P {
    variable marker first
    variable marker second
    set status [catch {variable item(k) value} message]
    list $marker $status $message
}]
variable rootdict(k) value
list $root $nsresult $rootdict(k)
