# naming.namespace.jim-original-scripted-helper-availability-and-forwarding

Kind: `native-observation`

## Problem statement

A namespace helper query must establish the actual public procedure-body capability before recording returned declarations or replacement calls. A version-variable existence guard supplies no such helper capability.

## Question

When the original public info-body guard reaches the Jim namespace helper, what formals/body results, global/current helper replacement values and caught unsupported/arity results are observed?

## Conclusion

Pinned Jim reports helper formals `subcommand args` and ensemble formals `command args`, with complete returned body values of 304 and 930 bytes. The separate global replacement returns `GLOBAL {create -map {go ::list}} ::N`; the namespace-local replacement returns `LOCAL {create -map {go ::list}} ::N`. The unsupported/arity source returns three caught failures: creation outside a namespace, unsupported configure and odd option count. All five C providers return the original NOT_APPLICABLE branch for each control. All 24 fresh original processes and six compilations exit 0 with empty stderr. The separately retained version-variable-gated source returns NOT_APPLICABLE in every one of its 24 controls, including Jim; it observes no helper body, replacement or forwarding. Full vendored helper/ensemble source files are separately pinned at 3917/1211 bytes, distinct from the returned public bodies. No stripping mechanism, private procedure token/header/frame, Native core bootstrap/compiler admission or successful arbitrary ensemble configuration follows. The linked Registry control compares the selected explicit NamespaceEnsemble library roster formals and complete body bytes with the actual public tuple. Linked Runtime and VM controls use all four untouched original sources and whole selected public completion fields for each of the six provider columns. This implementation binding grants no fresh native process result, continued helper binding, Native core bootstrap, compiler entry, private procedure token/header or frame. Current mutable helper lookup and explicit distribution installation remain separate from the public native observations.

Jim ensemble source schema retains exact create/-automap words and available source options. Abbreviations and C-only configuration operations are withheld together with compiler, handler, analyser, transition and world-effect metadata.

## Scope

Four exact original ASCII LF sources run independently in fresh full provider initialisation under the unchanged counted C API driver, without an added outer source envelope. Public info-body catch is part of the source itself. Whole sources/requests/driver/capture commands/ELFs/stdout/stderr/receipts and required executable/header/archive/source/build pins are retained. Jim public body values and full vendored source assets remain independent channels. The version-variable-gated four-control input and all its actual receipts are losslessly retained under limited-control; its original guard does not reach a helper call. BIG-IP and Rust assertions are not tested.

The selected source schema supplies neither an installed scripted helper nor successful forwarding, Native publication/frame/dispatch or Normal completion. Returned public helper bodies and current helper binding remain independent premises.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All four original public capability guards return NOT_APPLICABLE; no Jim helper body or replacement call is reached. The separately retained version-variable guards also return NOT_APPLICABLE. C namespace ensemble behaviour is not substituted for this Jim helper API.

### jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: jim 0.84-9-g5bac7c9.

Observed formals are `subcommand args` / `command args` with complete returned body values 304/930 bytes. Global/current replacements return GLOBAL/LOCAL with exact `{create -map {go ::list}} ::N` arguments/context. All three selected unsupported/arity calls are caught code1 with their complete messages retained. The independent version-variable guard returns NOT_APPLICABLE and reaches no helper call.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Pinned source tree; source-control revision associated with captured executable`, `/workspace/.proofs/native-providers/jimtcl/nshelper.tcl`, function `nshelper.tcl`, lines 1–159. Retained reference `native_jim_namespace_distribution236-request-nshelper.tcl`.

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

jim 0.84-9-g5bac7c9, revision `Pinned source tree; source-control revision associated with captured executable`, `/workspace/.proofs/native-providers/jimtcl/ensemble.tcl`, function `ensemble.tcl`, lines 1–36. Retained reference `native_jim_namespace_distribution236-request-ensemble.tcl`.

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

- [rust/tcl-registry/src/native_scripted_distribution.rs](../../../../rust/tcl-registry/src/native_scripted_distribution.rs), `procedures`: Explicit selected library roster; match original public formals/body values without claiming continued native command currency.
- [rust/tcl-registry/src/native_scripted_distribution.rs](../../../../rust/tcl-registry/src/native_scripted_distribution.rs), `InvocationDialect::native_namespace_scripted_helper`: Selected Jim namespace helper spelling delegates current script lookup, independently of core availability and C ensemble configuration.
- [rust/tcl-registry/src/native_scripted_distribution.rs](../../../../rust/tcl-registry/src/native_scripted_distribution.rs), `native_scripted_distribution::tests::original_namespace_library_metadata_matches_public_formals_and_bodies` (linked): The explicit Jim library roster matches both full formals/body values decoded from the actual original tuple; selected C and unknown Jim profiles supply no substitute library/protocol. Source metadata comparison is independent of continued native binding.
- [runtime/rust/src/cmd_namespace/native_holder_routing.rs](../../../../runtime/rust/src/cmd_namespace/native_holder_routing.rs), `cmd_namespace::native_holder_routing::original_jim_namespace_library_sources_match_all_native_provider_results` (linked): Use four byte-identical originals across all six public columns and compare complete original completion code/result fields under explicit provider initialisation; no reconstructed source, private frame or native process assertion.
- [rust/tcl-vm/src/interp/native_command_holder_tests.rs](../../../../rust/tcl-vm/src/interp/native_command_holder_tests.rs), `interp::native_command_holder_tests::original_jim_namespace_library_sources_match_all_native_provider_results` (linked): Use the same four untouched original scripts and six full public completion columns in VM; selected explicit library installation/current helper lookup stays separate from Native core and private protocol authority.

- [rust/tcl-registry/src/commands/tcl/namespace_.rs](../../../../rust/tcl-registry/src/commands/tcl/namespace_.rs), `jim_spec`: Model exact Jim helper source words and available options while withholding C compiler/state/effect descriptors and requiring separate current helper binding.
- [rust/tcl-registry/src/commands/tcl/namespace_.rs](../../../../rust/tcl-registry/src/commands/tcl/namespace_.rs), `commands::tcl::namespace_::tests::jim_scripted_ensemble_metadata_requires_exact_words_and_separate_helper_binding` (linked): Jim ensemble source schema accepts exact create/-automap words only and withholds abbreviated/C-only operations plus compiler, handler, analyser, transition and world-effect metadata. The helper binding remains independent. This is the current software/API definition; no assertion outcome or new original-provider observation is attached to this binding.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_namespace_distribution236](../../../../rust/tcl-registry/tests/data/native_jim_namespace_distribution236)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_jim_namespace_distribution236/limited-control](../../../../rust/tcl-registry/tests/data/native_jim_namespace_distribution236/limited-control)
