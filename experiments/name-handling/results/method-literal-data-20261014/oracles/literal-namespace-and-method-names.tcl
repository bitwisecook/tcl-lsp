if {[catch {info patchlevel} version]} {catch {info version} version}
puts [list version $version]
foreach name [list {} {$counter} {brace{name}} {a(b)} {::literal} {-dash}] {
    unset -nocomplain $name
    set declaration [catch {variable $name} declaration_message]
    set store [catch {set $name value} store_message]
    set read [catch {set $name} read_message]
    puts [list namespace-variable $name $declaration $declaration_message $store $store_message $read $read_message]
}
set class_result [catch {
    oo::class create LiteralNameProbe {
        method probe {} {
            variable {$counter}
            variable {}
            variable {brace{name}}
            set {$counter} dollar-value
            set {} empty-value
            set {brace{name}} brace-value
            return [list [set {$counter}] [set {}] [set {brace{name}}]]
        }
    }
    LiteralNameProbe create literalNameObject
    literalNameObject probe
} class_message]
puts [list method-literal-names $class_result $class_message]
