# TclOO configurable bootstrap and allocation

## Question

How does TclOO bootstrap configurable classes and resolve their definition namespaces, constructor and destructor behavior, and support-class lifetime? These distinctions affect class and namespace modeling in the LSP, compiler and VM. The script asks these specific questions:

- What namespace path is reported in each preexisting configurable definition namespace?

- Which own constructor and destructor definitions do source introspection queries return for the support and newly created classes, and how does the factory constructor query differ?

- Which factory, superclass and support mixin identities are reported for the literal configurable class created by this source?

- Does the created configurable class manufacture objects with zero arguments and with the exact two extra arguments one two?

- After the support command is renamed, does existing class manufacture continue while a fresh configurable factory call fails its original support lookup?

## Assumptions

The script checks whether `::oo::configurable` exists and stops if it does not. A missing configurable command does not establish that TclOO as a whole is unavailable. This is a public source-file control; it does not observe private object identity or a method-body frame. BIG-IP was not tested. Other assumptions were not separately recorded.

## Script

```tcl-visible
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
```

## Results

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
available⟦TAB⟧0⟦TAB⟧0
```

- Standard error: empty.

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
available⟦TAB⟧0⟦TAB⟧0
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
available⟦TAB⟧0⟦TAB⟧0
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
available⟦TAB⟧0⟦TAB⟧0
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
available⟦TAB⟧0⟦TAB⟧1
factory.class⟦TAB⟧0⟦TAB⟧::oo::class
factory.superclasses⟦TAB⟧0⟦TAB⟧::oo::class
factory.constructor⟦TAB⟧1⟦TAB⟧definition not available for this kind of method
support.class⟦TAB⟧0⟦TAB⟧::oo::class
support.constructor⟦TAB⟧0⟦TAB⟧
support.destructor⟦TAB⟧0⟦TAB⟧
support.superclasses⟦TAB⟧0⟦TAB⟧::oo::object
support.mixins⟦TAB⟧0⟦TAB⟧
support.methods⟦TAB⟧0⟦TAB⟧configure destroy
scope.class.path⟦TAB⟧0⟦TAB⟧::oo::define
scope.object.path⟦TAB⟧0⟦TAB⟧::oo::objdefine
create.class⟦TAB⟧0⟦TAB⟧::C
created.class⟦TAB⟧0⟦TAB⟧::oo::configurable
created.mixins⟦TAB⟧0⟦TAB⟧::oo::configuresupport::configurable
created.constructor⟦TAB⟧0⟦TAB⟧
created.destructor⟦TAB⟧0⟦TAB⟧
created.superclasses⟦TAB⟧0⟦TAB⟧::oo::object
created.filters⟦TAB⟧0⟦TAB⟧
default.new⟦TAB⟧0⟦TAB⟧::C
default.extra.arguments⟦TAB⟧0⟦TAB⟧::C
readable.properties⟦TAB⟧1⟦TAB⟧this command may only be called from within the context of an ::oo::define or ::oo::objdefine command
writable.properties⟦TAB⟧1⟦TAB⟧this command may only be called from within the context of an ::oo::define or ::oo::objdefine command
moved.support⟦TAB⟧0⟦TAB⟧::oo::Obj24
future.factory.missing.support⟦TAB⟧1⟦TAB⟧::oo::configuresupport::configurable does not refer to an object
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
available⟦TAB⟧0⟦TAB⟧1
factory.class⟦TAB⟧0⟦TAB⟧::oo::class
factory.superclasses⟦TAB⟧0⟦TAB⟧::oo::class
factory.constructor⟦TAB⟧1⟦TAB⟧definition not available for this kind of method
support.class⟦TAB⟧0⟦TAB⟧::oo::class
support.constructor⟦TAB⟧0⟦TAB⟧
support.destructor⟦TAB⟧0⟦TAB⟧
support.superclasses⟦TAB⟧0⟦TAB⟧::oo::object
support.mixins⟦TAB⟧0⟦TAB⟧
support.methods⟦TAB⟧0⟦TAB⟧configure destroy
scope.class.path⟦TAB⟧0⟦TAB⟧::oo::define
scope.object.path⟦TAB⟧0⟦TAB⟧::oo::objdefine
create.class⟦TAB⟧0⟦TAB⟧::C
created.class⟦TAB⟧0⟦TAB⟧::oo::configurable
created.mixins⟦TAB⟧0⟦TAB⟧::oo::configuresupport::configurable
created.constructor⟦TAB⟧0⟦TAB⟧
created.destructor⟦TAB⟧0⟦TAB⟧
created.superclasses⟦TAB⟧0⟦TAB⟧::oo::object
created.filters⟦TAB⟧0⟦TAB⟧
default.new⟦TAB⟧0⟦TAB⟧::C
default.extra.arguments⟦TAB⟧0⟦TAB⟧::C
readable.properties⟦TAB⟧1⟦TAB⟧this command may only be called from within the context of an ::oo::define or ::oo::objdefine command
writable.properties⟦TAB⟧1⟦TAB⟧this command may only be called from within the context of an ::oo::define or ::oo::objdefine command
moved.support⟦TAB⟧0⟦TAB⟧::oo::Obj24
future.factory.missing.support⟦TAB⟧1⟦TAB⟧::oo::configuresupport::configurable does not refer to an object
```

- Standard error: empty.
