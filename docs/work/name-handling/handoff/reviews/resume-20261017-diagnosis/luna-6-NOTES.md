# Luna-6 test corrections v1: assertion → probe → justification

Base: `/home/user/tcl-lsp` HEAD `84cf7c884` (branch `work`). `a/` holds pristine HEAD copies and `b/` the edited copies.
Patch: `test-corrections-v1.patch` (`diff -ruN a b`). `git -C /home/user/tcl-lsp apply --check` passes against the
working tree, where the definition.rs hunk applies at offset +19 over the uncommitted nextto production
candidate. `git apply --check --cached` also passes against the index (HEAD).
Authority: `user-decisions-20261017T2030Z.json`, decisions `document_links` and `non_tcl_tests`.
Nothing was compiled or run as a Rust test, because cargo was forbidden during the capture. The expected-value changes are
drafts until a gate run confirms them. `rustfmt --edition 2024 --check` is clean on all four edited files. call_hierarchy.rs and
code_lens.rs were checked in a throwaway copy that had an empty `logical_source.rs` stub.

Probes live in `probes/`. `run.sh P V…` writes `P.tclV.out` for each tclsh version. Interpreters: tclsh 8.4.20, 8.5.19,
8.6.18, 9.0.4 and 9.1.0. Every case runs in a fresh child interpreter. `probes/extract_fixtures.py b probes/fixtures` copies
the exact `src` literal of every changed TclOO/proc test from `b/` into `probes/fixtures/*.tcl`. `fixtures-exact.tcl` then
runs those exact bytes, prints the word under each test's cursor and prints the tclsh outcome. The document_links
probes embed the fixture strings verbatim.

## 1. document_links (decision `document_links`)

| Test (run #) | Changed assertion | Probe | Justification |
|---|---|---|---|
| `original_source_links_use_actual_selected_navigation_and_decline_shadows_or_stale_input` (test-124) | The fixture `"rename source load; load /tmp/é.tcl"` moves unchanged from the positive loop (`links.len() == 1`, target `file:///tmp/%C3%A9.tcl`, tooltip, x-replacement, stale-config checks) to the existing negative loop (`lexical_document_links_in_context(..).is_empty()`). | `probes/doclinks-124-rename-source-load.tcl` → `.tcl{8.4,8.5,8.6,9.0,9.1}.out` | On every version `rename source load` fails with `can't rename to "load": command already exists` (8.6+ errorCode `TCL OPERATION RENAME TARGET_EXISTS`). The script stops there, so `load` is never reached and the target file is not sourced. Even run alone, the original `load` of a `.tcl` file fails (`file too short`) and sources nothing. No file is sourced, so no link. |
| `double_dash_terminator_skipped` (test-140) | `assert_eq!(links.len(), 1)` and `assert_eq!(links[0].target, "file:///tmp/x.tcl")` become `assert!(links.is_empty(), "{links:?}")`. The fixture `source -- /tmp/x.tcl` is unchanged. | `probes/doclinks-140-source-double-dash.tcl` → `.tcl{8.4,8.5,8.6,9.0,9.1}.out` | `source` has no `--`. 8.4 gives `wrong # args: should be "source fileName"`. 8.5/8.6 give `wrong # args: should be "source ?-encoding name? fileName"` (8.6 errorCode `TCL WRONGARGS`). 9.0/9.1 give `bad option "--": must be -nopkg` (`TCL LOOKUP INDEX option --`). The 9.0 source is `tmp/tcl9.0.4/generic/tclCmdMZ.c:1133-1149`: with objc==3 the only accepted option is the undocumented `-nopkg`. The target is never sourced on any version. The control `source $target` does source it. |

Findings for the same probe that need no change:
- `::source f` and `interp alias {} load {} source; load f` do source the file on 8.4–9.1. The other two fixtures of that
  positive loop therefore stay as they are. Before this change, the loop never reached the alias fixture: it aborted on
  the rename fixture first.
- In the 9.0/9.1 `.out` files the probe label shows `é` as `�`. Only the printed label text is affected (stdout
  encoding). The literal case fails at `rename` before the path matters.
- `a_chained_computed_source_path_resolves_775` (test-136) is untouched, per the decision.

## 2. definition: nextto (decision `non_tcl_tests`; matches `i37/nextto/nextto-c-faithful-v2.patch`)

| Test | Change | Probe | Justification |
|---|---|---|---|
| `jump_to_nextto_namespaced_class_method` (existing) | `assert_eq!(locs.len(), 1)` and `assert_eq!(locs[0].start_line, 2, …)` become `assert!(locs.is_empty(), "{locs:?}")`. The comment now explains the lookup. The fixture and cursor `(10,28)` are unchanged. | `probes/nextto-namespaced.tcl` (`.tcl8.6/9.0/9.1.out`); exact fixture: `fixtures-exact.tcl*.out` | `[Ns::C new] greet` gives `A does not refer to an object` (`TCL LOOKUP OBJECT A`) on 8.6/9.0/9.1. A method body runs in the object namespace (`::oo::ObjNN`) with `namespace path` `::oo::Helpers`. The probe prints `::oo::Obj64 ::oo::Helpers`. Bare `A` therefore never reaches `::Ns::A`. With the v2 patch, the root lookup of `A` finds no `::A` and the path lookup finds no `::oo::Helpers::A`, so the result is `None`. |
| `jump_to_nextto_relative_qualified_class_method` (new) | Same fixture with `nextto Ns::A`. Expects 1 location on line 2 (`::Ns::A::greet`). | the same two probes | Returns `hi` on 8.6/9.0/9.1. In a variant where `::Ns::B::greet` returns `from-B` (both probes), the result is still `hi`. So the dispatch skips B and enters `::Ns::A::greet`, which is on line 2. In the v2 patch, `::oo::Helpers::Ns::A` is not a class, so there is no shadow. The root lookup of `Ns::A` gives `::Ns::A`. |
| `jump_to_nextto_fully_qualified_class_method` (new) | Same fixture with `nextto ::Ns::A`. Expects 1 location on line 2. | the same | Same `hi` / `hi` results. An absolute name skips the shadow check in the v2 patch. |
| `nextto_refuses_a_class_name_reached_through_the_helpers_path` (new) | The fixture defines `::oo::Helpers::A` (line 2 `greet`), root `::A` (line 6 `greet`), and `C` with `superclass ::A ::oo::Helpers::A` and `nextto A`. The cursor is `(10,22)`. Expects `locs.is_empty()`. | `probes/nextto-helpers-shadow.tcl` (`.tcl8.6/9.0/9.1.out`); exact fixture: `fixtures-exact.tcl*.out` | `[C new] greet` returns `helpers` on 8.6/9.0/9.1, both from `::` and from a nested eval. A helpers-only variant also returns `helpers`. Bare `A` resolves through the `::oo::Helpers` path before `::`, so a root answer (`::A::greet`, line 6) would be wrong. The v2 patch refuses this case (`shadowed` → `None`), and the test pins that refusal. If someone later chooses to answer here, tclsh says the answer must be line 2, never line 6. |

## 3. call_hierarchy (decision `non_tcl_tests`; Sol report group G7)

| Test | Choice | Change | Probe | Justification |
|---|---|---|---|---|
| `same_named_procs_in_different_namespaces_disambiguate_by_span` | Fixture fix. The intent is two real same-tail procs, and tclsh accepts that once the namespaces exist. | Line 0 gains the prefix `namespace eval a {}; namespace eval b {}; `. Lines 1–3, the cursor `(1,12)` and every assertion are unchanged. One comment line was added. | `probes/callhier-same-named-procs.tcl` (`.tcl8.6/9.0.out`); exact fixture: `fixtures-exact.tcl*.out` | The original fixture fails on its first command with `can't create procedure "::a::helper": unknown namespace` (`TCL VALUE COMMAND`). None of the four procs exist afterwards, so the scenario the test describes never happens. With the namespaces created, all four procs exist, `::b::helper` enters only `::bCallee` and `::a::helper` enters only `::aCallee`, which matches every assertion. Line 1 is byte-identical and character 12 is inside `helper`. Expecting "nothing" under the original fixture would erase the test's subject, so the fixture fix was chosen. |
| `prepare_resolves_classmethod_over_same_named_method_by_cursor` | Dialect fix. `classmethod` exists only in 9.0+. | `analyse(src)` → `analyse_tcl9(src)`, the existing helper in the same `mod tests`. A one-line doc note was added. The fixture, cursor `(2,16)` and assertions are unchanged. | `probes/callhier-classmethod.tcl` (`.tcl8.6/9.0/9.1.out`); exact fixture: `fixtures-exact.tcl*.out` | 8.6: `invalid command name "classmethod"` (`TCL LOOKUP COMMAND classmethod`), and `::C` does not exist afterwards. 9.0/9.1: `::C` exists, the delegate mixin holds `greet`, and `C greet` runs the classmethod body. The declaration under the cursor is therefore a real classmethod, and `classmethod of ::C (0 params)` holds. One caveat from the probe: in 9.0 this fixture's earlier `method greet` is replaced by `forward greet myclass greet`. The assertion does not depend on that. |
| `outgoing_calls_does_not_conflate_method_and_classmethod_sharing_a_name` | Dialect and fixture fix. The intent is an instance method and a classmethod that coexist, with `my greet` entering the instance method. tclsh 9.0 accepts that only when the classmethod is declared first. | `analyse` → `analyse_tcl9`. The fixture becomes `"oo::class create C {classmethod greet {} {}\n    method greet {} {}\n\n    method twice {} { my greet }\n}\n"`: the classmethod moves to line 0 and line 2 is left empty, so the instance method stays on line 1 and `twice` stays on line 3. The cursor `(3,11)`, `items[0].name == "::C::twice"`, `outgoing.len() == 1`, `from_ranges.len() == 1` and `start_line == 1` are all unchanged. Only comments changed: the doc note, and `classmethod (line 2)` → `(line 0)`. The `outgoing_calls` dialect stays the lenient `"tcl"`, as in the passing sibling tests. | the same probes | 8.6: the class is never created, as above. 9.0/9.1 with the original order: `C greet` is `FORWARD to {myclass greet}`, and the instrumented `[C new] twice` returns `classmethod`. So in tclsh the original fixture's `my greet` reaches the classmethod, which contradicts both the doc comment and the line-1 assertion. 9.0/9.1 with the proposed fixture: `info class definition C greet` is a real method, the delegate still has `greet`, the instrumented `[C new] twice` returns `instance-method` and `C greet` returns `classmethod`. That is exactly the "independent tables" scenario the test describes. Alternative, not taken: swap lines 1 and 2. That reads more naturally but changes the asserted `start_line` from 1 to 2, so it does not keep every value. |

## 4. code_lens absence-as-completeness tests (decision `non_tcl_tests`)

The tests asserted `"0 references"`, which is a complete-census claim. HEAD returns `"References unavailable"` (`reference_coverage: Partial`).

| Test | Changed assertion | Probe | Justification |
|---|---|---|---|
| `lens_bare_word_in_method_body_is_not_a_self_dispatch` (test-057) | `"0 references"` → `"References unavailable"`. A comment was added. The fixture is unchanged. | `probes/lens-bare-word.tcl` (`.tcl8.6/9.0.out`) | As written, `[C new] other` gives `invalid command name "greet"` (`TCL LOOKUP COMMAND greet`) on 8.6/9.0, and the method is entered 0 times. So the bare words are not self-dispatch, which confirms the FP guard: they must never be counted. The same unchanged document enters `greet` twice when the embedding supplies `proc ::unknown {args} {uplevel 1 [list my {*}$args]}` or `proc ::greet {} {uplevel 1 {my greet}}`. The site is an unknown command, and its effect depends on commands the document does not define. Claiming `0 references` would mean inventing completeness. The tclsh-faithful lens is partial and counts nothing. This matches the unknown-command uncertainty in fix-5 (G5). |
| `lens_unknown_instance_method_is_not_counted` (test-067) | `"0 references"` → `"References unavailable"`. A comment was added. The fixture is unchanged. | `probes/lens-unknown-receiver.tcl` (`.tcl8.6/9.0.out`) | As written, `$x bark` gives `can't read "x": no such variable` (`TCL LOOKUP VARNAME x`), and `bark` is entered 0 times, so the site must not count. When the embedding presets `x` to a proxy (`proc ::proxy {method args} {[Dog new] $method {*}$args}; set x ::proxy`), the same document enters `Dog::bark` once. With `x` set to an unrelated command, it is entered 0 times. The receiver depends on state the document does not set, so a complete `0` is not supported by tclsh. The lens stays partial. |

Both expected values equal HEAD's current output, so the tests would turn green without a production change.
A future sound method census (decision `method_census_contradiction`, targets 056/058) should leave both of them
Partial, because each fixture keeps an unresolved site. 058 has no such site.

## 5. Left untouched (findings only)

- `incoming_calls_for_an_itcl_class_proc_find_colon_qualified_dispatch` and
  `outgoing_calls_from_an_itcl_class_proc_reach_the_sibling_class_proc`, probed with `probes/notes-itcl.tcl` (`.tcl8.6/9.0.out`).
  As written, without `package require`, the fixture fails on 8.6/9.0 with `invalid command name "itcl::class"`. No Itcl is
  installed for the repo's tclsh builds (`can't find package itcl`). The only Itcl build on the machine is the Google Cloud
  SDK's bundled Tcl-9 build (itcl 4.3.5). With `ITCL_LIBRARY` set, tclsh 9.0.4 loads it, and the fixture behaves as the test's
  oracle says: `Factory::build`, `[Factory #auto] viaInstance` and the top-level `Factory::make` all enter `make` (3 entries).
  The tests analyse at tcl8.6 with no `package require`. Whether the analyser should assume Itcl without a require is a
  policy question, not a tclsh contradiction of the edge expectations.
- `consumed_command_names_remain_references_without_becoming_calls`, probed with `probes/notes-consumed-name.tcl` (`.tcl8.6/9.0.out`).
  `info body target` reads `::target`'s body without entering it (0 entries). It rejects a non-proc (`"set" isn't a
  procedure`). It resolves the name from the caller's namespace (inside `::n` it reads `::n::target`). The fixture enters
  `target` exactly once, through the bare call. That is consistent with the incoming/outgoing assertions (1 edge, 1 range).
  The failing assertion is `ConsumedName` vs `PossibleConsumedName`, which is an internal lookup-kind classification that
  tclsh cannot distinguish. No correction is proposed.
