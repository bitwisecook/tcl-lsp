# naming.ensemble.original-counted-map-prefix-construction

Kind: `native-observation`

## Problem statement

A map prefix is an original counted String value. Applying later command-lookup extent while constructing its namespace-qualified head can discard a NUL suffix before the map is stored or publicly queried. Create and configure also select different entered namespaces.

## Question

Does original namespace ensemble create/configure preserve a counted raw-NUL String map-prefix head when namespace-qualifying and reading back the map, independently from later command lookup, in the actual C8.4–9.1 and current initialized Jim providers?

## Conclusion

Six actual compiles and thirty fresh initialized host processes complete with exit0 and empty stderr. All twenty C8.5, C8.6, C9.0 and C9.1 windows retain the raw NUL and suffix in the counted STORED_HEAD and complete MAP_QUERY. A relative head is qualified in namespace N during create, namespace Q during configure, or the global namespace in the global create control; the rooted head keeps namespace N. Each separate CALL returns N FIRST, Q FIRST or GLOBAL FIRST as selected, despite the retained NUL suffix in the queried head. C8.4 supplies actual unsupported ensemble errors; the initialized Jim image supplies actual scripted-factory arity, root-namespace and configure rejection errors for these C-style vectors. Their whole setup/input/operation/query/call results are retained and do not supply a common C map-construction observation. Source excerpts independently show the relative-head construction branches; no private object primary, pointer, holder or software test execution is established.

## Scope

Exactly five original object-vector controls per six pinned providers. The unchanged callback constructs the head with Tcl_NewStringObj or Jim_NewStringObj and an explicit raw byte count, then original prefix/map list objects, inside the actual entered namespace. It does not substitute a ByteArray, binary command or source-parsed NUL. OPERATION, counted STORED_HEAD readback, complete MAP_QUERY and later CALL are independent output fields. Eight pinned C create/configure source excerpts explain selected construction branches separately from those observations. The complete request, probe, launcher, thirty process receipts/stdout/stderr, six actual compile receipts/ELFs, fourteen additional whole source copies, inherited whole source/build metadata and exact required pins are retained. No arbitrary map, invalid encoding, private identity/refcount, Jim factory equivalence, body/frame/compiler admission, BIG-IP or executed Rust assertion claim follows. NativeNameProtocol::ensemble_map_prefix_input selects the C8.5–9.1 counted construction purpose independently of command lookup. Rooted input is borrowed unchanged; relative input appends every original byte to the actual namespace report with a separator selected by the independently supplied actual parent fact. Runtime and VM adapters obtain report and parent from their retained actual namespace owner, then preserve the original map/list recipe. These API inputs do not establish a physical holder from reporting text. The marked pure Syntax control and both complete20-window backend controls are definition bindings only: every non-version original SETUP/INITIAL/INPUT/OPERATION/STORED_HEAD/MAP_QUERY/CALL row is compared independently. C8.4/Jim unsupported originals remain unchanged evidence, without common C map-worker authority. The separate unmarked unavailable-purpose/parent-fact control is authored software policy and supplies no native detached-holder observation. No new software pass or private original object/lookup identity is claimed.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh initialized actual String/list object vectors in an entered namespace; counted map readback and later call measured separately. Version: tcl8.4 8.4.20.

All five whole originals report caught unsupported ensemble errors for OPERATION and MAP_QUERY, and invalid command name ::r2286_map_E for CALL. The two configure originals also retain the INITIAL ensemble rejection. INPUT remains the original counted String bytes; no STORED_HEAD is emitted and no successful map construction is observed. Five fresh host processes exit0 with empty stderr. Complete streams and receipts retain every original field; source inspection and software coverage are independent.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh initialized actual String/list object vectors in an entered namespace; counted map readback and later call measured separately. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All five original operations and queries return0. Create-relative and create-rooted read back ::r2286_map_N::p NUL tail; configure-relative reads back ::r2286_map_Q::p NUL tail; configure-rooted retains namespace N; global create-relative reads back ::p NUL tail. The counted heads have lengths21 or8, and complete MAP_QUERY retains the same raw00 suffix. Independent CALL returns N FIRST, Q FIRST or GLOBAL FIRST as selected. OPERATION retains the original create command result versus the empty configure result. Five fresh host processes exit0 with empty stderr. Complete streams and receipts retain every original field; source inspection and software coverage are independent.

### jim

Status: `observed`. Channel: Fresh initialized actual String/list object vectors in an entered namespace; counted map readback and later call measured separately. Version: jim 0.84-9-g5bac7c9.

All five whole originals retain the actual initialized Jim version and setup result. Entered-namespace C-style create vectors report wrong # args for the scripted ensemble factory; the global create vector reports namespace ensemble create: must be called within a namespace. Configure and every MAP_QUERY report only [namespace ensemble create] is supported. The two configure originals also retain the INITIAL factory arity error. Each CALL reports invalid command name ::r2286_map_E. No STORED_HEAD or Jim equivalent of the C map worker is observed. Five fresh host processes exit0 with empty stderr. Complete streams and receipts retain every original field; source inspection and software coverage are independent.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

tcl8.5 Pinned original 8.5.19 create/configure construction branch; separate source inspection., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceEnsembleCmd`, lines 4908–4927. Retained reference `native_ensemble_map_prefix312-map-source-8.5.19-tclNamesp.c`.

```text
		    }
		    cmd = TclGetString(listv[0]);
		    if (!(cmd[0] == ':' && cmd[1] == ':')) {
			Tcl_Obj *newList = Tcl_NewListObj(len, listv);
			Tcl_Obj *newCmd = Tcl_NewStringObj(nsPtr->fullName,-1);

			if (nsPtr->parentPtr) {
			    Tcl_AppendStringsToObj(newCmd, "::", NULL);
			}
			Tcl_AppendObjToObj(newCmd, listv[0]);
			Tcl_ListObjReplace(NULL, newList, 0, 1, 1, &newCmd);
			if (patchedDict == NULL) {
			    patchedDict = Tcl_DuplicateObj(objv[1]);
			}
			Tcl_DictObjPut(NULL, patchedDict, subcmdObj, newList);
		    }
		    Tcl_DictObjNext(&search, &subcmdObj, &listObj, &done);
		} while (!done);

		if (allocatedMapFlag) {

```

tcl8.5 Pinned original 8.5.19 create/configure construction branch; separate source inspection., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceEnsembleCmd`, lines 5181–5200. Retained reference `native_ensemble_map_prefix312-map-source-8.5.19-tclNamesp.c`.

```text
			}
			cmd = TclGetString(listv[0]);
			if (!(cmd[0] == ':' && cmd[1] == ':')) {
			    Tcl_Obj *newList = Tcl_NewListObj(len, listv);
			    Tcl_Obj *newCmd =
				    Tcl_NewStringObj(nsPtr->fullName, -1);
			    if (nsPtr->parentPtr) {
				Tcl_AppendStringsToObj(newCmd, "::", NULL);
			    }
			    Tcl_AppendObjToObj(newCmd, listv[0]);
			    Tcl_ListObjReplace(NULL, newList, 0,1, 1,&newCmd);
			    if (patchedDict == NULL) {
				patchedDict = Tcl_DuplicateObj(objv[1]);
			    }
			    Tcl_DictObjPut(NULL, patchedDict, subcmdObj,
				    newList);
			}
			Tcl_DictObjNext(&search, &subcmdObj, &listObj, &done);
		    } while (!done);
		    if (allocatedMapFlag) {

```

tcl8.6 Pinned original 8.6.18 create/configure construction branch; separate source inspection., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEnsemble.c`, function `TclNamespaceEnsembleCmd`, lines 288–307. Retained reference `native_ensemble_map_prefix312-map-source-8.6.18-tclEnsemble.c`.

```text
		    }
		    cmd = TclGetString(listv[0]);
		    if (!(cmd[0] == ':' && cmd[1] == ':')) {
			Tcl_Obj *newList = Tcl_NewListObj(len, listv);
			Tcl_Obj *newCmd = NewNsObj((Tcl_Namespace *) nsPtr);

			if (nsPtr->parentPtr) {
			    Tcl_AppendStringsToObj(newCmd, "::", (char *)NULL);
			}
			Tcl_AppendObjToObj(newCmd, listv[0]);
			Tcl_ListObjReplace(NULL, newList, 0, 1, 1, &newCmd);
			if (patchedDict == NULL) {
			    patchedDict = Tcl_DuplicateObj(objv[1]);
			}
			Tcl_DictObjPut(NULL, patchedDict, subcmdWordsObj,
				newList);
		    }
		    Tcl_DictObjNext(&search, &subcmdWordsObj, &listObj, &done);
		} while (!done);

```

tcl8.6 Pinned original 8.6.18 create/configure construction branch; separate source inspection., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEnsemble.c`, function `TclNamespaceEnsembleCmd`, lines 571–590. Retained reference `native_ensemble_map_prefix312-map-source-8.6.18-tclEnsemble.c`.

```text
			}
			cmd = TclGetString(listv[0]);
			if (!(cmd[0] == ':' && cmd[1] == ':')) {
			    Tcl_Obj *newList = Tcl_DuplicateObj(listObj);
			    Tcl_Obj *newCmd = NewNsObj((Tcl_Namespace*)nsPtr);

			    if (nsPtr->parentPtr) {
				Tcl_AppendStringsToObj(newCmd, "::", (char *)NULL);
			    }
			    Tcl_AppendObjToObj(newCmd, listv[0]);
			    Tcl_ListObjReplace(NULL, newList, 0, 1, 1, &newCmd);
			    if (patchedDict == NULL) {
				patchedDict = Tcl_DuplicateObj(objv[1]);
			    }
			    Tcl_DictObjPut(NULL, patchedDict, subcmdWordsObj,
				    newList);
			}
			Tcl_DictObjNext(&search, &subcmdWordsObj, &listObj,
				&done);
		    } while (!done);

```

tcl9.0 Pinned original 9.0.4 create/configure construction branch; separate source inspection., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEnsemble.c`, function `InitEnsembleFromOptions`, lines 343–362. Retained reference `native_ensemble_map_prefix312-map-source-9.0.4-tclEnsemble.c`.

```text
		cmd = TclGetString(listv[0]);
		if (!(cmd[0] == ':' && cmd[1] == ':')) {
		    Tcl_Obj *newList = Tcl_NewListObj(len, listv);
		    Tcl_Obj *newCmd = TclNewNamespaceObj(
			    (Tcl_Namespace *) nsPtr);

		    if (nsPtr->parentPtr) {
			Tcl_AppendStringsToObj(newCmd, "::", (char *)NULL);
		    }
		    Tcl_AppendObjToObj(newCmd, listv[0]);
		    Tcl_ListObjReplace(NULL, newList, 0, 1, 1, &newCmd);
		    if (patchedDict == NULL) {
			patchedDict = Tcl_DuplicateObj(objv[1]);
		    }
		    Tcl_DictObjPut(NULL, patchedDict, subcmdWordsObj, newList);
		}
		Tcl_DictObjNext(&search, &subcmdWordsObj, &listObj, &done);
	    } while (!done);

	    if (allocatedMapFlag) {

```

tcl9.0 Pinned original 9.0.4 create/configure construction branch; separate source inspection., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEnsemble.c`, function `SetEnsembleConfigOptions`, lines 669–688. Retained reference `native_ensemble_map_prefix312-map-source-9.0.4-tclEnsemble.c`.

```text
		cmd = TclGetString(listv[0]);
		if (!(cmd[0] == ':' && cmd[1] == ':')) {
		    Tcl_Obj *newList = Tcl_DuplicateObj(listObj);
		    Tcl_Obj *newCmd = TclNewNamespaceObj(
			    (Tcl_Namespace*) nsPtr);

		    if (nsPtr->parentPtr) {
			Tcl_AppendStringsToObj(newCmd, "::", (char *)NULL);
		    }
		    Tcl_AppendObjToObj(newCmd, listv[0]);
		    Tcl_ListObjReplace(NULL, newList, 0, 1, 1, &newCmd);
		    if (patchedDict == NULL) {
			patchedDict = Tcl_DuplicateObj(objv[1]);
		    }
		    Tcl_DictObjPut(NULL, patchedDict, subcmdWordsObj, newList);
		}
		Tcl_DictObjNext(&search, &subcmdWordsObj, &listObj, &done);
	    } while (!done);
	    if (allocatedMapFlag) {
		Tcl_DecrRefCount(mapObj);

```

tcl9.1 Pinned original 9.1.0 create/configure construction branch; separate source inspection., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEnsemble.c`, function `InitEnsembleFromOptions`, lines 343–362. Retained reference `native_ensemble_map_prefix312-map-source-9.1.0-tclEnsemble.c`.

```text
		cmd = TclGetString(listv[0]);
		if (!(cmd[0] == ':' && cmd[1] == ':')) {
		    Tcl_Obj *newList = Tcl_NewListObj(len, listv);
		    Tcl_Obj *newCmd = TclNewNamespaceObj(
			    (Tcl_Namespace *) nsPtr);

		    if (nsPtr->parentPtr) {
			Tcl_AppendStringsToObj(newCmd, "::", (char *)NULL);
		    }
		    Tcl_AppendObjToObj(newCmd, listv[0]);
		    Tcl_ListObjReplace(NULL, newList, 0, 1, 1, &newCmd);
		    if (patchedDict == NULL) {
			patchedDict = Tcl_DuplicateObj(objv[1]);
		    }
		    Tcl_DictObjPut(NULL, patchedDict, subcmdWordsObj, newList);
		}
		Tcl_DictObjNext(&search, &subcmdWordsObj, &listObj, &done);
	    } while (!done);

	    if (allocatedMapFlag) {

```

tcl9.1 Pinned original 9.1.0 create/configure construction branch; separate source inspection., revision `Exact original content SHA; no source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEnsemble.c`, function `SetEnsembleConfigOptions`, lines 669–688. Retained reference `native_ensemble_map_prefix312-map-source-9.1.0-tclEnsemble.c`.

```text
		cmd = TclGetString(listv[0]);
		if (!(cmd[0] == ':' && cmd[1] == ':')) {
		    Tcl_Obj *newList = Tcl_DuplicateObj(listObj);
		    Tcl_Obj *newCmd = TclNewNamespaceObj(
			    (Tcl_Namespace*) nsPtr);

		    if (nsPtr->parentPtr) {
			Tcl_AppendStringsToObj(newCmd, "::", (char *)NULL);
		    }
		    Tcl_AppendObjToObj(newCmd, listv[0]);
		    Tcl_ListObjReplace(NULL, newList, 0, 1, 1, &newCmd);
		    if (patchedDict == NULL) {
			patchedDict = Tcl_DuplicateObj(objv[1]);
		    }
		    Tcl_DictObjPut(NULL, patchedDict, subcmdWordsObj, newList);
		}
		Tcl_DictObjNext(&search, &subcmdWordsObj, &listObj, &done);
	    } while (!done);
	    if (allocatedMapFlag) {
		Tcl_DecrRefCount(mapObj);

```

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::ensemble_map_prefix_input`: Construct the selected C8.5+ map head with all counted original bytes; actual namespace parent is an independent caller premise, with later command lookup separate.
- [runtime/rust/src/cmd_namespace.rs](../../../../runtime/rust/src/cmd_namespace.rs), `native_ensemble_map_prefix_input`: Read actual retained namespace report and parent from Runtime and delegate counted map construction to the shared purpose.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::native_ensemble_map_prefix_input`: Retain actual VM namespace identity while independently supplying its report and physical-parent fact to the shared counted construction purpose.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::ensemble_map_prefix_construction_preserves_counted_heads_before_lookup` (linked): Pure selected C8.5–9.1 construction retains raw NUL suffixes in relative and rooted heads, independently of the later command-lookup projection. Report/parent inputs are authored API premises and do not prove a native detached holder or original header.
- [runtime/rust/src/cmd_namespace/native_map_prefix_tests.rs](../../../../runtime/rust/src/cmd_namespace/native_map_prefix_tests.rs), `cmd_namespace::native_map_prefix_tests::original_counted_map_prefix_vectors_match_all_20_c_public_windows` (linked): Authentic selected C8.5/8.6/9.0/9.1 counted String/list vectors compare all20 complete original setup/initial/input/operation/stored-head/map-query/call public windows. Only VERSION metadata is excluded explicitly. C8.4/Jim rejection streams remain separate retained observations; no private primary/pointer, physical holder or software execution pass is inferred.
- [rust/tcl-vm/src/cmd_namespace/native_map_prefix_tests.rs](../../../../rust/tcl-vm/src/cmd_namespace/native_map_prefix_tests.rs), `cmd_namespace::native_map_prefix_tests::original_counted_map_prefix_vectors_match_all_20_c_public_windows` (linked): Authentic selected C8.5/8.6/9.0/9.1 counted String/list vectors compare all20 complete original setup/initial/input/operation/stored-head/map-query/call public windows. Only VERSION metadata is excluded explicitly. C8.4/Jim rejection streams remain separate retained observations; no private primary/pointer, physical holder or software execution pass is inferred.

## Evidence

- [rust/tcl-registry/tests/data/native_ensemble_map_prefix312](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/request/map-source](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/request/map-source)
