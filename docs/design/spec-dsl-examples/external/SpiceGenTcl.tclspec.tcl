# Draft spec for georgtree/SpiceGenTcl -- the issue #1363 reporter's OWN
# library. Source: github.com/georgtree/SpiceGenTcl @ e8aa45ce (2026-08-11),
# package version 0.71, `package require Tcl 9.0-` (SpiceGenTcl.tcl:17).
#
# Exercises: object_class (INVENTED, G1) on an argparse-driven constructor
# whose real shape is invisible from the outside (G4), option_constraints
# for `-forbid` (INVENTED syntax, real struct backing) alongside an
# invented `requires` clause with NO struct backing at all (G2), a
# zero-method alias-via-subclass needing no new syntax at all, an
# argument-shape ensemble via `-key/-value` argparse flags (same family as
# ticklecharts::chart's Add and apave's widgetType — see README §5 item 2),
# and method-level taint sinks that cannot actually be declared today
# (documents G7 with real exec/open-pipe evidence).
#
# NOT drafted, deliberately: `oo::configurable`'s `property NAME -get
# {BODY} -set {BODY}` clause (generalClasses.tcl:342-344,365-374, 45
# `oo::configurable create` / 63 `property` declarations total). This is a
# `definition_body`-grammar shape, not per-command data — `fields.md`:
# "Grammars are shared, named descriptors... the studio cannot author the
# grammar inline." `property`'s shape (zero param-list, up to two OPTIONAL
# named body attachments `-get`/`-set`, an implicit `$value` inside `-set`)
# doesn't fit `definition_body`'s documented `{name, params, body}` triple
# regardless — already named abstractly in ../tricky-surfaces.md:28
# ("oo::configurable's property, which is also 9.0-gated per member"); this
# corpus is that item's first concrete grounding (G12).
#
# Full evidence and gap numbering: ../external/README.md

speclib SpiceGenTcl 0.71 {

# ---------------------------------------------------------------------------
# ::SpiceGenTcl::Ngspice::BasicDevices::Resistor
# (specElementsClassesNgspice.tcl:34-135). Constructor is `{args}` from the
# outside — arity 0..inf tells a caller nothing. The REAL shape is an
# `argparse -inline -pfirst { ... }` block (specElementsClassesNgspice.tcl:
# 90-107) with 3 mandatory positionals and 13 flags carrying `-forbid`/
# `-require` relationships. Transcribed here by hand — see G4 for why that
# transcription cannot be automated today (164 such blocks in this corpus).
command ::SpiceGenTcl::Ngspice::BasicDevices::Resistor {
    arity 3..
    dialects {tcl9.0 tcl9.1}
    required_package SpiceGenTcl

    arg 0 -role Name  -detail {Device name, without the leading "R" designator.}
    arg 1 -role Value -detail {Node connected to the positive pin.}
    arg 2 -role Value -detail {Node connected to the negative pin.}

    option -r     -takes value -detail {Resistance value or equation.}
    option -beh                -detail {Behavioural (equation-driven) resistor.}
    option -model -takes value -detail {Model card name (semiconductor resistor).}
    option -ac    -takes value -detail {AC resistance value.}
    option -l     -takes value -detail {Length of a semiconductor resistor.}
    option -w     -takes value -detail {Width of a semiconductor resistor.}
    option -noisy -takes value -values {0 1} -closed -detail {Selects noise behaviour.}

    # specElementsClassesNgspice.tcl:92: `{-beh -forbid {model} -require {r}
    # ...}`; :102-103: `{-l= -require {model} ...} {-w= -require {model}
    # ...}`. `forbid` maps onto the real `OptionConstraint`
    # (rust/tcl-registry/src/spec.rs:520-525, `{options: &[&str], dialects}`
    # -- a flat "may not co-occur" set); no sibling `.tclspec.tcl` in the
    # parent directory shows `option_constraints` syntax yet either, so
    # this spelling is new even for the *backed* half. `requires` is
    # INVENTED with NO struct backing whatsoever (G2) -- 189 real
    # occurrences of `-require` across this corpus, vs 75 `-forbid`.
    option_constraints {
        forbid {-beh -model}
        forbid {-ac -model} {-ac -beh}
        forbid {-m -beh}
        requires {-l -w} {-model}   ;# DSL GAP (G2): no struct field backs this at all.
        requires {-beh} {-r}        ;# DSL GAP (G2): ditto.
    }

    hover {
        summary  {Describe a resistor for SPICE netlist generation.}
        synopsis {Resistor new name np nm -r value ?-tc1 v? ?-tc2 v? ?-ac v? ...}
        synopsis {Resistor new name np nm -model value ?-l v? ?-w v? ...}
        description {Simple, behavioural (-beh, equation-driven), or model-card (-model, semiconductor) resistor. The constructor computes the SPICE designator name (`r$name`) and calls `next` into `::SpiceGenTcl::Device` with the assembled parameter list (specElementsClassesNgspice.tcl:133) -- an ordinary TCLOO_NEXT_CHAIN use, no gap.}
        source   {specElementsClassesNgspice.tcl:34-135}
    }

    object_class {
        # ::SpiceGenTcl::Device (drafted below) supplies genSPICEString,
        # actOnParam, actOnPin, checkFloatingPins, etc. via ordinary TclOO
        # inheritance -- `superclasses` already covers this with zero new
        # fields.
        superclasses {::SpiceGenTcl::Device}
        allow_unknown_methods no
    }
}

# ---------------------------------------------------------------------------
# ::SpiceGenTcl::Ngspice::BasicDevices::R (specElementsClassesNgspice.tcl:
# 138-140) -- the ENTIRE class body is `superclass Resistor`. A pure
# spelling alias (the SPICE element-letter designator) implemented as an
# empty subclass rather than a Tcl `interp alias`/`rename`. Included to show
# this needs NOTHING beyond `superclasses` and an empty method list --
# `ObjectClassSpec.superclasses` (spec.rs:259-261) already resolves
# inherited methods, exactly as its own doc comment promises.
command ::SpiceGenTcl::Ngspice::BasicDevices::R {
    arity 3..
    dialects {tcl9.0 tcl9.1}
    required_package SpiceGenTcl
    hover { summary {Alias for Resistor (the "R" SPICE element-letter designator).} source {specElementsClassesNgspice.tcl:137-140} }
    object_class {
        superclasses {::SpiceGenTcl::Ngspice::BasicDevices::Resistor}
        allow_unknown_methods no
        # No `method` entries: R contributes nothing of its own: every
        # method (including the constructor's argument shape above,
        # inherited via `superclass`, not restated) comes from Resistor.
    }
}

# ---------------------------------------------------------------------------
# ::SpiceGenTcl::Device (generalClasses.tcl:649-978, superclass of all 200+
# leaf element classes) -- `actOnParam` is the argument-shape ensemble: an
# argparse `-key action -value X` block maps `-add`/`-get`/`-set`/`-delete`/
# `-all` onto a single `action` variable (generalClasses.tcl:822-835), then
# `switch -- $action { add {...} get {...} set {...} delete {...} }`
# (generalClasses.tcl:837-908). Same underlying need as ticklecharts::
# chart's `Add` and apave's `widgetType` -- three different source-level
# idioms (literal switch, char-prefix switch, argparse `-key/-value` flags)
# converging on the exact same DSL requirement (README §5 item 2).
command ::SpiceGenTcl::Device {
    arity 0..
    dialects {tcl9.0 tcl9.1}
    required_package SpiceGenTcl
    hover { summary {Base class for every two-or-more-pin circuit element.} source {generalClasses.tcl:649-978} }

    object_class {
        superclasses {}
        allow_unknown_methods no

        method actOnParam {
            arity 1..
            detail   {Add, get, set, or delete one of the device's SPICE parameters.}
            synopsis {$dev actOnParam -add ?-pos|-eq|...? name value}
            synopsis {$dev actOnParam -get name}
            synopsis {$dev actOnParam -set name value ?name value ...?}
            synopsis {$dev actOnParam -delete name}
            mutator

            option -add    -detail {Add a new parameter (requires: name, value). generalClasses.tcl:822.}
            option -get    -detail {Get a parameter's value (requires: name, or -all). generalClasses.tcl:823.}
            option -set    -detail {Set (or change) one or more parameters. generalClasses.tcl:824.}
            option -delete -detail {Delete an existing parameter. generalClasses.tcl:825.}
            option -all    -detail {With -get: return every parameter as a name/value dict. generalClasses.tcl:826.}

            # generalClasses.tcl:822 `{-add -key action -value add -require
            # pname ...}` -- one more `-require` instance, same G2 gap as
            # Resistor's constructor above; repeated here to show it is not
            # confined to constructors.
            option_constraints {
                forbid {-add -get} {-add -set} {-add -delete}
                forbid {-get -set} {-get -delete}
                forbid {-set -delete}
                requires {-add} {pname}        ;# DSL GAP (G2)
                requires {-all} {-get}         ;# DSL GAP (G2)
            }

            hover { summary {The device's parameter-table CRUD surface, dispatched by mutually-exclusive option flags rather than a subcommand word.} source {generalClasses.tcl:795-908} }
        }
    }
}

# ---------------------------------------------------------------------------
# ::SpiceGenTcl::Ngspice::Simulators::Batch / BatchLiveLog
# (specSimulatorClassesNgspice.tcl:21-103, 105-147) -- real command-
# injection-shaped sinks INSIDE TclOO instance methods, not free commands.
# `runAndRead` also builds a FILE PATH directly from the untrusted
# `circuitStr` argument's first line. This is the sharpest evidence for G7:
# the taint fields these methods actually need
# (taint_code_sink_args/taint_network_sink_args) are *command only* per
# fields.md, and these are unambiguously SubCommand-shaped (object_class
# instance methods).
command ::SpiceGenTcl::Ngspice::Simulators::Batch {
    arity 1..2
    dialects {tcl9.0 tcl9.1}
    required_package SpiceGenTcl
    arg 0 -role Name -detail {Simulator object name.}
    arg 1 -role Value -detail {Run/output directory (default ".").}
    hover { summary {Batch (non-interactive) ngspice simulator.} source {specSimulatorClassesNgspice.tcl:21-58} }

    object_class {
        superclasses {::SpiceGenTcl::Simulator}
        allow_unknown_methods no

        method runAndRead {
            arity 1..2
            detail   {Write circuitStr to a .cir file, run ngspice on it, and read back the results.}
            synopsis {$sim runAndRead circuitStr ?-nodelete?}
            mutator
            arg 0 -role Value -detail {Top-level SPICE netlist text.}
            option -nodelete -detail {Keep the generated .cir/.raw/.log files instead of deleting them.}

            # specSimulatorClassesNgspice.tcl:71,74-76: `[file join
            # $runLocation ${firstLine}.cir]` where firstLine is literally
            # `[lindex [split $circuitStr \n] 0]` -- a FILE PATH built from
            # caller-supplied text with no validation. Legal today (side
            # effects ARE command-and-subcommand scoped) --
            side_effect FileIo -reads -writes

            # -- but specSimulatorClassesNgspice.tcl:77: `exec {*}[list
            # $Command -b -r $rawFileName -o $logFileName $cirFileName]` is
            # a genuine process-spawn sink, and there is NO SubCommand
            # field to declare it as one:
            #
            #   taint_code_sink_args {0}      ;# DSL GAP (G7) -- command-only field, cannot attach to this method.
            #   side_effect Process -writes   ;# `Process` IS a valid SideEffectTarget (fields.md), so at least
            #                                  ;# this half is legal -- included for completeness:
            side_effect Process -writes

            hover { summary {Run the netlist through ngspice in batch mode.} source {specSimulatorClassesNgspice.tcl:59-86} }
        }
    }
}

command ::SpiceGenTcl::Ngspice::Simulators::BatchLiveLog {
    arity 1..2
    dialects {tcl9.0 tcl9.1}
    required_package SpiceGenTcl
    hover { summary {Batch ngspice simulator that streams its log live.} source {specSimulatorClassesNgspice.tcl:105-147} }

    object_class {
        # Inherits everything from Batch except runAndRead, which it
        # OVERRIDES with a different sink shape -- same method name, real
        # polymorphism, no `next` call (specSimulatorClassesNgspice.tcl:
        # 108-146 is a full replacement, not an extension).
        superclasses {::SpiceGenTcl::Ngspice::Simulators::Batch}
        allow_unknown_methods no

        method runAndRead {
            arity 1..2
            detail   {Like Batch::runAndRead, but pipes ngspice's stdout/stderr live instead of writing a .log file.}
            synopsis {$sim runAndRead circuitStr ?-nodelete?}
            mutator
            arg 0 -role Value

            # specSimulatorClassesNgspice.tcl:127-128: `set command [list
            # $Command -b $cirFileName -r $rawFileName]; set chan [open
            # "|$command 2>@1"]` -- the `open |cmd` shell-pipe idiom,
            # structurally the same risk class as `exec` even though Tcl's
            # own list-to-argv rule (via `[list ...]`) makes this specific
            # call shell-injection-resistant in practice. Same gap as
            # Batch::runAndRead above (G7):
            #   taint_code_sink_args {0}   ;# DSL GAP (G7)
            side_effect Process   -writes
            side_effect ChannelIo -reads -writes

            hover { summary {Run ngspice, streaming its combined stdout/stderr through an open pipe.} source {specSimulatorClassesNgspice.tcl:108-146} }
        }
    }
}

}
