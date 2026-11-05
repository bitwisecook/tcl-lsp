# naming.namespace.jim-original-scripted-ensemble-publication-with-rooted-result

Kind: `native-observation`

## Problem statement

A factory result stored by an unqualified namespace-source variable command is not necessarily readable through a namespace-qualified variable spelling. Positive publication/dispatch requires a source that reaches the result and target calls.

## Question

With the created result retained through an explicit rooted variable, which original Jim ensemble spelling, callable target/formals/command-query values and exact selector/option rejections are observed?

## Conclusion

The original rooted-result Jim controls return `N {DEFAULT VALUE} {subcmd args} go`, `N {TARGET VALUE} {{}}` and `N {PREFIX VALUE} go` for default, namespace-prefix and literal-command-prefix inputs. The literal-prefix source targets the single command named "target go" through its supplied trailing-space prefix. The independent selector/option control catches code1 for abbreviated creation (only [namespace ensemble create] is supported) and for -map (the original ensemble command ?-automap prefix? usage). The command-query payload {{}} stays an exact observed value; it is not classified as absent mapping. All five C providers return their original NOT_APPLICABLE branch. All24 main processes and six compiler commands exit0 with empty stderr. Separately retained inputs that read ::N::made finish with original code1/no-such-variable for all three Jim factory controls, so those final completions cannot donate created-name or target-call results; their selector/option observation remains code0 with the same two caught failures. No private mapping/table/token/header/frame, general factory acceptance, C ensemble equivalence or compiler/Normal authority follows. The linked Registry control models only exact Jim create/-automap source schema and requires independent helper binding; it retains no inherited C compiler/state/analyser/effect descriptors. Runtime and VM controls compare all four unchanged rooted-result originals against all six public columns. That authored schema/current lookup and source replay grant no fresh native process, private mapping/table/frame, arbitrary factory binding or successful Normal completion.

## Scope

Four exact original ASCII LF sources run separately through the unchanged fresh full-init counted C API driver. The explicit rooted result variable is part of each original positive input, and the standalone selector/option source retains exact caught errors. All original sources/requests/drivers/launchers/ELFs/streams/receipts and required provider/header/archive/build/source pins are retained. The independently authored namespace-qualified-result inputs and all actual compiler/process captures remain under limited-control with their measured original outcomes. Full Jim namespace/helper/ensemble source files remain source-inspection evidence separate from public result/dispatch values. BIG-IP and Rust assertions are not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All four main and all four independently retained namespace-qualified-result sources return ORIGINAL0/NOT_APPLICABLE. C ensemble behaviour is not substituted for the Jim scripted API.

### jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: jim 0.84-9-g5bac7c9.

Whole ORIGINAL0 values: default-namespace-command=N {DEFAULT VALUE} {subcmd args} go; namespace-prefix-override=N {TARGET VALUE} {{}}; literal-command-prefix-override=N {PREFIX VALUE} go; exact-create-and-option-selection=1 {only [namespace ensemble create] is supported} 1 {wrong # args: should be "ensemble command ?-automap prefix?"}. The separate ::N::made-result originals have code1/no-such-variable in three factory controls and no positive call result; exact rejection control remains independently measured.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Pinned original Jim provider source`, `/workspace/.proofs/native-providers/jimtcl/nshelper.tcl`, function `nshelper.tcl`, lines 1–159. Retained reference `native_jim_ensemble_rooted_result245-request-nshelper.tcl`.

```text
# Implements script-based implementations of various namespace
# subcommands
#
# (c) 2011 Steve Bennett <steveb@workware.net.au>
#

proc {namespace delete} {args} {
	foreach name $args {
		if {$name ni {:: ""}} {
			set name [uplevel 1 [list ::namespace canon $name]]
			foreach i [info commands ${name}::*] { rename $i "" }
			uplevel #0 [list unset {*}[info globals ${name}::*]]
		}
	}
}

proc {namespace origin} {name} {
	set nscanon [uplevel 1 [list ::namespace canon $name]]
	if {[exists -alias $nscanon]} {
		tailcall {namespace origin} [info alias $nscanon]
	}
	if {[exists -command $nscanon]} {
		return ::$nscanon
	}
	if {[exists -command $name]} {
		return ::$name
	}

	return -code error "invalid command name \"$name\""
}

proc {namespace which} {{type -command} name} {
	set nsname ::[uplevel 1 [list ::namespace canon $name]]
	if {$type eq "-variable"} {
		return $nsname
	}
	if {$type eq "-command"} {
		if {[exists -command $nsname]} {
			return $nsname
		} elseif {[exists -command ::$name]} {
			return ::$name
		}
		return ""
	}
	return -code error {wrong # args: should be "namespace which ?-command? ?-variable? name"}
}

proc {namespace code} {arg} {
	if {[string first "::namespace inscope " $arg] == 0} {
		# Already scoped
		return $arg
	}
	list ::namespace inscope [uplevel 1 ::namespace current] $arg
}

proc {namespace inscope} {name arg args} {
	tailcall namespace eval $name $arg $args
}

proc {namespace import} {args} {
	set current [uplevel 1 ::namespace canon]

	foreach pattern $args {
		foreach cmd [info commands [namespace canon $current $pattern]] {
			if {[namespace qualifiers $cmd] eq $current} {
				return -code error "import pattern \"$pattern\" tries to import from namespace \"$current\" into itself"
			}
			# What if this alias would create a loop?
			# follow the target alias chain to see if we are creating a loop
			set newcmd ${current}::[namespace tail $cmd]

			set alias $cmd
			while {[exists -alias $alias]} {
				set alias [info alias $alias]
				if {$alias eq $newcmd} {
					return -code error "import pattern \"$pattern\" would create a loop"
				}
			}

			alias $newcmd $cmd
		}
	}
}

# namespace-aware info commands: procs, channels, globals, locals, vars
proc {namespace info} {cmd {pattern *}} {
	set current [uplevel 1 ::namespace canon]
	# Now we may need to strip $pattern
	if {[string first :: $pattern] == 0} {
		set global 1
		set prefix ::
	} else {
		set global 0
		set clen [string length $current]
		incr clen 2
	}
	set fqp [namespace canon $current $pattern]
	switch -glob -- $cmd {
		co* - p* {
			if {$global} {
				set result [info $cmd $fqp]
			} else {
				# Add commands in the current namespace
				set r {}
				foreach c [info $cmd $fqp] {
					dict set r [string range $c $clen end] 1
				}
				if {[string match co* $cmd]} {
					# Now in the global namespace
					foreach c [info -nons commands $pattern] {
						dict set r $c 1
					}
				}
				set result [dict keys $r]
			}
		}
		ch* {
			set result [info channels $pattern]
		}
		v* {
			#puts "uplevel #0 info gvars $fqp"
			set result [uplevel #0 info -nons vars $fqp]
		}
		g* {
			set result [info globals $fqp]
		}
		l* {
			set result [uplevel 1 info -nons locals $pattern]
		}
	}
	if {$global} {
		set result [lmap p $result { string cat $prefix $p }]
	}
	return $result
}

proc {namespace upvar} {ns args} {
	set nscanon ::[uplevel 1 [list ::namespace canon $ns]]
	set script [list upvar 0]
	foreach {other local} $args {
		lappend script ${nscanon}::$other $local
	}
	tailcall {*}$script
}

proc {namespace ensemble} {subcommand args} {
	if {$subcommand ne "create"} {
		return -code error "only \[namespace ensemble create\] is supported"
	}
	set ns [uplevel 1 namespace canon]
	set cmd $ns
	if {$ns eq ""} {
		return -code error "namespace ensemble create: must be called within a namespace"
	}

	# Create the mapping
	ensemble $cmd -automap ${ns}:: {*}$args
}

```

jim 0.84-9-g5bac7c9, revision `Pinned original Jim provider source`, `/workspace/.proofs/native-providers/jimtcl/ensemble.tcl`, function `ensemble.tcl`, lines 1–36. Retained reference `native_jim_ensemble_rooted_result245-request-ensemble.tcl`.

```text
# Implement the ensemble command

proc ensemble {command args} {
	set autoprefix "$command "
	set badopts "should be \"ensemble command ?-automap prefix?\""
	if {[llength $args] % 2 != 0} {
		return -code error "wrong # args: $badopts"
	}
	foreach {opt value} $args {
		switch -- $opt {
			-automap { set autoprefix $value }
			default { return -code error "wrong # args: $badopts" }
		}
	}
	proc $command {subcmd args} {autoprefix {mapping {}}} {
		if {![dict exists $mapping $subcmd]} {
			# Not an exact match, so check for specials, then lookup normally
			if {$subcmd in {-commands -help}} {
				# Need to remove $autoprefix from the front of these
				set prefixlen [string length $autoprefix]
				set subcmds [lmap p [lsort [info commands -all $autoprefix*]] {
					string range $p $prefixlen end
				}]
				if {$subcmd eq "-commands"} {
					return $subcmds
				}
				set command [lindex [info level 0] 0]
				return "Usage: \"$command command ... \", where command is one of: [join $subcmds ", "]"
			}
			# cache the mapping
			dict set mapping $subcmd ${autoprefix}$subcmd
		}
		# tailcall here we don't add an extra stack frame, e.g. for uplevel
		tailcall [dict get $mapping $subcmd] {*}$args
	}
}

```

## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/namespace_.rs](../../../../rust/tcl-registry/src/commands/tcl/namespace_.rs), `jim_spec`: Separate exact authored Jim scripted create/-automap metadata from C ensemble compiler/state/effect purposes and actual helper binding.
- [rust/tcl-registry/src/commands/tcl/namespace_.rs](../../../../rust/tcl-registry/src/commands/tcl/namespace_.rs), `commands::tcl::namespace_::tests::jim_scripted_ensemble_metadata_requires_exact_words_and_separate_helper_binding` (linked): Exact create/-automap authored schema refuses prefix/unsupported selectors and inherited C purposes; independent helper binding remains necessary. No installed native factory/current frame is proved.
- [runtime/rust/src/cmd_namespace/native_holder_routing.rs](../../../../runtime/rust/src/cmd_namespace/native_holder_routing.rs), `cmd_namespace::native_holder_routing::original_jim_ensemble_sources_match_all_native_rooted_public_results` (linked): Use all four byte-identical rooted-result originals and six whole native public columns, preserving original created spelling/targets/rejection values; no private mapping or reconstructed source.
- [rust/tcl-vm/src/interp/native_command_holder_tests.rs](../../../../rust/tcl-vm/src/interp/native_command_holder_tests.rs), `interp::native_command_holder_tests::original_jim_ensemble_sources_match_all_native_rooted_public_results` (linked): VM consumes the same four originals and six complete public code/result columns, independently of Native core/frame/compiler or private mapping authority.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_ensemble_rooted_result245](../../../../rust/tcl-registry/tests/data/native_jim_ensemble_rooted_result245)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_jim_ensemble_rooted_result245/limited-control](../../../../rust/tcl-registry/tests/data/native_jim_ensemble_rooted_result245/limited-control)
