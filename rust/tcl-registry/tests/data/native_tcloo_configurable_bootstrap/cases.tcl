# Public source controls for the configurable bootstrap and default manufacture.
proc row {id script} {
    set code [catch {uplevel 1 $script} value]
    puts [join [list $id $code $value] \t]
}
row available {expr {[llength [info commands ::oo::configurable]] == 1}}
if {[llength [info commands ::oo::configurable]] == 0} {
    return
}
row factory.class {info object class ::oo::configurable}
row factory.superclasses {info class superclasses ::oo::configurable}
row factory.constructor {info class constructor ::oo::configurable}
row support.class {info object class ::oo::configuresupport::configurable}
row support.constructor {info class constructor ::oo::configuresupport::configurable}
row support.destructor {info class destructor ::oo::configuresupport::configurable}
row support.superclasses {info class superclasses ::oo::configuresupport::configurable}
row support.mixins {info class mixins ::oo::configuresupport::configurable}
row support.methods {lsort [info class methods ::oo::configuresupport::configurable -all]}
row scope.class.path {namespace eval ::oo::configuresupport::configurableclass {namespace path}}
row scope.object.path {namespace eval ::oo::configuresupport::configurableobject {namespace path}}
row create.class {::oo::configurable create ::C {property readable -kind readable writable -kind writable}}
row created.class {info object class ::C}
row created.mixins {info class mixins ::C}
row created.constructor {info class constructor ::C}
row created.destructor {info class destructor ::C}
row created.superclasses {info class superclasses ::C}
row created.filters {info class filters ::C}
row default.new {set object [::C new]; info object class $object}
row default.extra.arguments {set extra [::C new one two]; info object class $extra}
row readable.properties {namespace eval ::oo::configuresupport {readableproperties ::C}}
row writable.properties {namespace eval ::oo::configuresupport {writableproperties ::C}}
row moved.support {rename ::oo::configuresupport::configurable ::HeldSupport; ::C new}
row future.factory.missing.support {::oo::configurable create ::Later {}}
