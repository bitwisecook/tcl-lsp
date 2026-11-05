# naming.info.original-namespace-helper-declaration-and-current-forwarding

Kind: `native-observation`

## Problem statement

A namespace-aware command inventory needs evidence for the currently selected mutable scripted helper, its public declaration and actual original caller/vector routing. A synthetic name list or rooted written head alone cannot supply those facts.

## Question

What exact helper formals/body and caller/vector reports do the original namespace info declaration and replacement controls produce, and which original ordinary, rooted, root-pattern, root-all and explicit -nons calls reach the replacement?

## Conclusion

Jim returns the complete two-field public declaration tuple of 929 bytes: formals cmd {pattern *} and a 909-byte body. The independently retained nshelper.tcl source has a 1059-byte declaration body; the public returned body and raw source are separate exact artifacts, and no transformation mechanism is inferred. In the nonroot replacement original, ordinary info and rooted ::info both report HELPER ::r2286_holder {commands -all r2286*}; explicit -nons instead returns {::r2286_global r2286_holder::r2286_local}. At actual root, a single qualified pattern reports HELPER :: {commands ::r2286*}; root -all and explicit -nons bypass the authored replacement and the original filtered returned values are both empty. All five C releases return NOT_APPLICABLE in each of these helper-capability originals. All18 fresh original processes and six compiler commands exit0 with empty stderr. These public authored helper reports establish the reached replacement vectors and namespace reports for these exact controls, without proving private argument/header identity, current command-table/token/cache identity, Native frame allocation or general stock inventory equivalence. A literal multiword namespace info procedure does not establish a namespace-core info subcommand.

A marked shared VM definition binds the native_jim_info_helper_forwarding258 public subset (18 comparisons) independently of its other fixture groups.

## Scope

Three unchanged ASCII LF originals run through the unchanged counted-source driver with fresh full provider initialisation. Their exact source/request/driver/launcher/ELF bytes, whole command receipts/streams and required archive/header/build/source pins are retained. The public returned formal/body tuple retains every tab, blank line and byte; the independently inspected source declaration is not substituted for that tuple. No outer catch/eval/puts envelope is added. Other helper behaviour, arbitrary Unicode/NUL, private object/cache/identity, compiler or Normal admission, BIG-IP and executed Rust assertions are not measured. Linked Runtime and VM controls execute the unchanged source fixtures and compare all138 whole ORIGINAL completion/result windows across the four inventory/helper groups per backend, with private identity and META/version excluded from the behavioural result. Their linked status records source coverage, not an executed software pass. The Registry asset control compares the entire public two-field declaration tuple through the shared native list parser. Its full909-byte body is kept as the selected NativeScriptedLibrary asset; the current mutable helper lookup remains independent from authentic Core startup and actual library installation. Broad C/other-helper/core presence is not inferred from that asset.

The shared body has138 public code/result comparisons across23 unchanged originals, but each question owns only its matching fixture group. It selects an actual software core/compiler and explicitly installs NamespaceEnsemble; this does not infer a full distribution, current helper roster, token/cache identity, native opcode or entered external handler. It launches no new original process and supplies no assertion outcome. Existing independent original channels remain unchanged.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Whole ORIGINAL code/value fields: original-helper-formals-and-body=0/NOT_APPLICABLE; original-helper-replacement-caller-and-vector=0/NOT_APPLICABLE; original-root-qualified-pattern-forwards-current-helper=0/NOT_APPLICABLE. These original helper-capability guards are not applicable; no equivalent helper substitution is executed.

### jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: jim 0.84-9-g5bac7c9.

Whole ORIGINAL code/value fields: original-helper-formals-and-body=0/{cmd {pattern *}} {
	set current [uplevel 1 ::namespace canon]

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

				set r {}
				foreach c [info $cmd $fqp] {
					dict set r [string range $c $clen end] 1
				}
				if {[string match co* $cmd]} {

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
}; original-helper-replacement-caller-and-vector=0/{HELPER ::r2286_holder {commands -all r2286*}} {HELPER ::r2286_holder {commands -all r2286*}} {::r2286_global r2286_holder::r2286_local}; original-root-qualified-pattern-forwards-current-helper=0/{HELPER :: {commands ::r2286*}} {} {}. Public declaration body and separately inspected raw provider source remain distinct. The measured replacement reports unchanged written vectors and its own actual caller namespace report; no private argv/header/frame identity follows.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

jim 0.84-9-g5bac7c9, revision `Pinned independent original provider source`, `/workspace/.proofs/native-providers/jimtcl/nshelper.tcl`, function `proc {namespace info}`, lines 87–159. Retained reference `native_jim_info_helper_forwarding258-request-source-anchors-jim-nshelper.tcl`.

```text
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

## Consumer bindings

- [runtime/rust/src/cmd_info/native_jim_inventory_tests.rs](../../../../runtime/rust/src/cmd_info/native_jim_inventory_tests.rs), `cmd_info::native_jim_inventory_tests::original_jim_inventory_and_current_helper_sources_match_all_native_columns` (linked): Authentic selected native bootstrap executes all23 untouched originals across six providers and compares all138 full ORIGINAL code/result windows/backend, excluding host refusal. Original nonroot forwarding, root/nons inventory and current-helper declaration/replacement stay independently identified; no private table/header/frame claim.
- [rust/tcl-registry/src/native_scripted_distribution.rs](../../../../rust/tcl-registry/src/native_scripted_distribution.rs), `native_scripted_distribution::tests::original_namespace_info_library_matches_public_declaration` (linked): Shared list split over whole exact captured929-byte tuple compares original public formals and all909 body bytes with the selected library asset, preserving empty lines/tabs; raw nshelper source and actual mutable command binding remain separate.

- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `info_command_listing`: Validate the actual original worker argv and reached usage rewrite before delegating public inventory to the shared command-list purpose; selected helper/library installation remains independent.
- [rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs](../../../../rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs), `original_result`: Decode the exact original ORIGINAL completion/result-hex window; no private inventory/header observation follows from this parser.
- [rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs](../../../../rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs), `cmd_info::native_jim_inventory_tests::original_jim_inventory_and_current_helper_sources_match_all_native_columns` (linked): Three original helper declaration/replacement/root-forwarding programs retain public formals/body and current caller/effective-vector behavior; this is not a helper private object or full linked-library proof. This question binds only the native_jim_info_helper_forwarding258 subset: 18 of the shared body's138 public comparisons.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258](../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/request/source-anchors/jim](../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/request/source-anchors/jim)
