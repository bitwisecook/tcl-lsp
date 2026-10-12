# naming.procedure.original-root-and-colon-holder-publication

Kind: `native-observation`

## Problem statement

A source model can collapse a literal root Procedure name beginning with one colon, or treat Procedure holder publication and a lambda namespace spelling as the same name operation. The actual release-specific public results and independently inspected producer recipes must remain distinct.

## Question

In the exact safe-child source, where is a root :source procedure visible, where does holder :ns publish its source procedure, and which namespace does a literal lambda with explicit :ns or omitted namespace select?

## Conclusion

All five C releases keep the root :source procedure distinct from the hidden plain source command. C8.4/8.5 publish a Procedure declared under :ns into ns; C8.6/9.0/9.1 retain it under :ns. Tcl8.4 cannot execute Apply. Tcl8.5 and later select ns for a lambda explicitly naming :ns, while an omitted lambda namespace selects root. Jim rejects the child surface and supplies no colon-holder or lambda namespace answer.

## Scope

Six fresh CLI processes retain six actual version rows and 78 sequential caught observations. The thirteen controls within each process share creation and procedure state; independent catch completion does not mean independent interpreter state. Results measure finite public names, inventories, caught arity/missing-command outcomes and errorCode values only. Two exact C8.4/C8.5 publisher windows and four C8.5–C9.1 lambda windows are independent source-inspection evidence. Original requested source-anchor metadata is retained unchanged; four Lambda ranges are replaced only in the separate LF-normalized joins. Neither source inspection nor CLI results measure input objects, native tokens, pointer/epoch correspondence, physical namespace/cell allocation, arbitrary constructor Normal, reached runtime frame, compiler admission or BIG-IP behavior. Linked Rust API coverage separates the counted lambda namespace object prefix from subsequent CString lookup extent. A conditional child-source fixture keeps C8.5 non-global procedure holder reparsing distinct from C8.6+ literal holder publication; it supplies no child frame/cell, selected callable or runtime execution observation.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual CLI executable SHA f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a with retained SDK/header/library/build/source pins, environment and runner associations. The CLI runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI; one fresh provider process with thirteen sequential caught controls in the same source state after its queried version row.. Dialect: Tcl.

Root :source is callable as :source while the hidden plain source remains unavailable. The procedure declared under holder :ns is visible and callable under ns, and absent from :ns. Apply is unavailable, so the two lambda controls provide no lambda namespace answer. Missing source/apply commands retain errorCode NONE.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual CLI executable SHA e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a with retained SDK/header/library/build/source pins, environment and runner associations. The CLI runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI; one fresh provider process with thirteen sequential caught controls in the same source state after its queried version row.. Dialect: Tcl.

Root :source remains distinct from hidden plain source. Holder :ns publishes its source procedure under ns. The explicit lambda namespace :ns selects that ns procedure and reaches its arity failure; the omitted lambda namespace selects root and cannot call hidden source. These failures retain errorCode NONE.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual CLI executable SHA 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff with retained SDK/header/library/build/source pins, environment and runner associations. The CLI runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI; one fresh provider process with thirteen sequential caught controls in the same source state after its queried version row.. Dialect: Tcl.

Root :source remains distinct from hidden plain source. Holder :ns retains its source procedure under :ns, and ns has none. Calling it with a.tcl reaches TCL WRONGARGS. The explicit lambda namespace :ns selects ns and cannot find source; the omitted namespace selects root and likewise cannot find the hidden command, with TCL LOOKUP COMMAND source.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual CLI executable SHA f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1 with retained SDK/header/library/build/source pins, environment and runner associations. The CLI runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI; one fresh provider process with thirteen sequential caught controls in the same source state after its queried version row.. Dialect: Tcl.

Root :source remains distinct from hidden plain source. Holder :ns retains its source procedure under :ns, and ns has none. Calling it with a.tcl reaches TCL WRONGARGS. The explicit lambda namespace :ns selects ns and cannot find source; the omitted namespace selects root and likewise cannot find the hidden command, with TCL LOOKUP COMMAND source.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual CLI executable SHA 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d with retained SDK/header/library/build/source pins, environment and runner associations. The CLI runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI; one fresh provider process with thirteen sequential caught controls in the same source state after its queried version row.. Dialect: Tcl.

Root :source remains distinct from hidden plain source. Holder :ns retains its source procedure under :ns, and ns has none. Calling it with a.tcl reaches TCL WRONGARGS. The explicit lambda namespace :ns selects ns and cannot find source; the omitted namespace selects root and likewise cannot find the hidden command, with TCL LOOKUP COMMAND source.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual CLI executable SHA e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806 with retained SDK/header/library/build/source pins, environment and runner associations. The CLI runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI; one fresh provider process with thirteen sequential caught controls in the same source state after its queried version row.. Dialect: Jim Tcl.

Every tested child-interpreter form fails with the actual interp arity message. These failures supply no Jim colon-holder, Procedure publication or lambda namespace answer; they do not establish that the bare interp command is absent.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation of these exact root and colon-holder publication controls.

## Exact evidence

- `colon-source-anchors.json` (input): [rust/tcl-registry/tests/data/native_child_colon_publication/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/source-anchors.json). SHA-256 `c2c45fd33cc0eb59839bb5fcddd0da3a65b972a2cc0f45be295e0a46cc19453c`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-probe.tcl` (input): [rust/tcl-registry/tests/data/native_child_colon_publication/probe.tcl](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/probe.tcl). SHA-256 `b97802e5b2f167443153158cec9c1d752d284121dbde6328444b9403bf009227`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-request.json` (input): [rust/tcl-registry/tests/data/native_child_colon_publication/request.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/request.json). SHA-256 `ed30fd3686a23a21fcb4e6c7ae08e10cd6452b270548bcad29d5b1eeb9fcf76b`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-capture.py` (input): [rust/tcl-registry/tests/data/native_child_colon_publication/capture.py](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/capture.py). SHA-256 `0a548e2f4e8dfbe789b5f5b6f5877f72f17d2f7212b94707252d7d6a87a9a006`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_colon_publication/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.6.18/receipt.json). SHA-256 `3dbaab0f5c54b29353b7ae4a2d4164b1e618d09cb73af5236bf37da2baa0de52`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.6.18/stdout). SHA-256 `c8fc71457dd4bfb0435650841e7f6ff174d1485b1dc1a92c474901a7a016d50e`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_colon_publication/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/9.1.0/receipt.json). SHA-256 `af887755c0c8a4fedcec7063c2aa373be7576fb11ac523d793f531e78b2d8c5a`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/9.1.0/stdout). SHA-256 `7f78344a4481d483d259ae23a13541f9071b58034c496a15bdf9e8b7a5cc1085`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_colon_publication/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.4.20/receipt.json). SHA-256 `4c27a97547f13dd9b1d78c8f4ce23b412be6634af9ecf422610567b328103b70`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.4.20/stdout). SHA-256 `c897b452f1b2d091cc60e185d8734512e035a19e99c1de2423fcb3ec9c12e701`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_colon_publication/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/9.0.4/receipt.json). SHA-256 `58dcabf658585342721edd60e4517f8567ed61a708ff5510aae329829bdb701e`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/9.0.4/stdout). SHA-256 `9c32dfb92615fdcbdb237cd6fe25b4ac2c24d4f7cfa4bcf09418eba1311b77f9`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_colon_publication/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.5.19/receipt.json). SHA-256 `e80558eaf065838ac1b262cf8315ebf0f69a7803e9961424ab590f7b5c523ad4`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/8.5.19/stdout). SHA-256 `b06ec6ca3df08410b9c1023e50ed67073cba9cf75f645f3888454c9108f8d362`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_colon_publication/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/jim/receipt.json). SHA-256 `9b4ae8fcc5ba078d675c86c07d23e0be848f35586536fe9acb7aea865ba22f05`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/jim/stderr](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_child_colon_publication/jim/stdout](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/jim/stdout). SHA-256 `24b78349ba3a9b1570823aa59941d97c6e7465a10f742f25e2b6f8b2af7dbf98`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-sdk-queue.json` (provider): [rust/tcl-registry/tests/data/native_child_colon_publication/sdk-queue.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/sdk-queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-full-source-8.4.20-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/full-source/8.4.20/tclProc.c](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/full-source/8.4.20/tclProc.c). SHA-256 `bfeecc7c08dabce16946efad9c0e7eb3cacf43d4a5376bf622fa4b69483eb879`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-full-source-8.5.19-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/full-source/8.5.19/tclProc.c](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/full-source/8.5.19/tclProc.c). SHA-256 `9adc0419de1af0c81c7718e1e682fb50f4e2873f7b2d07a0f3cbe380f7d2dce6`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-full-source-8.6.18-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/full-source/8.6.18/tclProc.c](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/full-source/8.6.18/tclProc.c). SHA-256 `c1ddd801a69b0e39bf923e97838134489ca4937fb7d081eccc0814b6811f71b3`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-full-source-9.0.4-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/full-source/9.0.4/tclProc.c](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/full-source/9.0.4/tclProc.c). SHA-256 `ef5ef608fbb22c68d35317e09637f099380457e5d01ff8e1b67f2a8eeb70c110`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-full-source-9.1.0-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/full-source/9.1.0/tclProc.c](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/full-source/9.1.0/tclProc.c). SHA-256 `2c5cf8968a3176aa8e5c316063594186f50109cecd2fbe3c3dff2f3035cefd1a`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-source-windows.json` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json). SHA-256 `85bf826e6dbb310c336257f03bdfb0659a31cd6134171316ff5b3edaae1286ce`. Exact retained source request/runner, original requested anchor metadata, independently inspected source bytes, SDK association or original CLI provider stream. Requested metadata ranges are not source evidence coordinates; separate normalized windows identify exact LF ranges.
- `colon-source-window-0` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json). SHA-256 `85bf826e6dbb310c336257f03bdfb0659a31cd6134171316ff5b3edaae1286ce`. JSON pointer `/windows/0/snippet`. Exact independently inspected Tcl_ProcObjCmd bytes and LF coordinates joined to the retained complete release source digest.
- `colon-source-window-1` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json). SHA-256 `85bf826e6dbb310c336257f03bdfb0659a31cd6134171316ff5b3edaae1286ce`. JSON pointer `/windows/1/snippet`. Exact independently inspected Tcl_ProcObjCmd bytes and LF coordinates joined to the retained complete release source digest.
- `colon-source-window-2` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json). SHA-256 `85bf826e6dbb310c336257f03bdfb0659a31cd6134171316ff5b3edaae1286ce`. JSON pointer `/windows/2/snippet`. Exact independently inspected SetLambdaFromAny bytes and LF coordinates joined to the retained complete release source digest.
- `colon-source-window-3` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json). SHA-256 `85bf826e6dbb310c336257f03bdfb0659a31cd6134171316ff5b3edaae1286ce`. JSON pointer `/windows/3/snippet`. Exact independently inspected SetLambdaFromAny bytes and LF coordinates joined to the retained complete release source digest.
- `colon-source-window-4` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json). SHA-256 `85bf826e6dbb310c336257f03bdfb0659a31cd6134171316ff5b3edaae1286ce`. JSON pointer `/windows/4/snippet`. Exact independently inspected SetLambdaFromAny bytes and LF coordinates joined to the retained complete release source digest.
- `colon-source-window-5` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/source-windows.json). SHA-256 `85bf826e6dbb310c336257f03bdfb0659a31cd6134171316ff5b3edaae1286ce`. JSON pointer `/windows/5/snippet`. Exact independently inspected SetLambdaFromAny bytes and LF coordinates joined to the retained complete release source digest.

## Source inspection

tcl8.4 8.4.20, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl8.4.20/generic/tclProc.c`, function `Tcl_ProcObjCmd`, lines 126–143. Full-source SHA-256 `bfeecc7c08dabce16946efad9c0e7eb3cacf43d4a5376bf622fa4b69483eb879`; snippet SHA-256 `b100249a9a6566a9c52959c969fc2a14e3cc289476daae77768fa5e65012eb3d`; retained evidence `colon-source-window-0`.

```text
     * Now create a command for the procedure. This will initially be in
     * the current namespace unless the procedure's name included namespace
     * qualifiers. To create the new command in the right namespace, we
     * generate a fully qualified name for it.
     */

    Tcl_DStringInit(&ds);
    if (nsPtr != iPtr->globalNsPtr) {
	Tcl_DStringAppend(&ds, nsPtr->fullName, -1);
	Tcl_DStringAppend(&ds, "::", 2);
    }
    Tcl_DStringAppend(&ds, procName, -1);

    Tcl_CreateCommand(interp, Tcl_DStringValue(&ds), TclProcInterpProc,
	    (ClientData) procPtr, TclProcDeleteProc);
    cmd = Tcl_CreateObjCommand(interp, Tcl_DStringValue(&ds),
	    TclObjInterpProc, (ClientData) procPtr, TclProcDeleteProc);


```

tcl8.5 8.5.19, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl8.5.19/generic/tclProc.c`, function `Tcl_ProcObjCmd`, lines 173–187. Full-source SHA-256 `9adc0419de1af0c81c7718e1e682fb50f4e2873f7b2d07a0f3cbe380f7d2dce6`; snippet SHA-256 `42a3cb36df01b6f23da0a3773062d1d6317d4dbcd098a889a40a6b9c86ad9f34`; retained evidence `colon-source-window-1`.

```text
     * Now create a command for the procedure. This will initially be in the
     * current namespace unless the procedure's name included namespace
     * qualifiers. To create the new command in the right namespace, we
     * generate a fully qualified name for it.
     */

    Tcl_DStringInit(&ds);
    if (nsPtr != iPtr->globalNsPtr) {
	Tcl_DStringAppend(&ds, nsPtr->fullName, -1);
	Tcl_DStringAppend(&ds, "::", 2);
    }
    Tcl_DStringAppend(&ds, procName, -1);

    cmd = Tcl_CreateObjCommand(interp, Tcl_DStringValue(&ds),
	    TclObjInterpProc, (ClientData) procPtr, TclProcDeleteProc);

```

tcl8.5 8.5.19, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl8.5.19/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2591–2602. Full-source SHA-256 `9adc0419de1af0c81c7718e1e682fb50f4e2873f7b2d07a0f3cbe380f7d2dce6`; snippet SHA-256 `77acf3444c81b456c1385d5c18b486bd8382e17772d7286c7c02504d9ca4a203`; retained evidence `colon-source-window-2`.

```text
    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	char *nsName = TclGetString(objv[2]);

	if ((*nsName != ':') || (*(nsName+1) != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

```

tcl8.6 8.6.18, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl8.6.18/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2569–2580. Full-source SHA-256 `c1ddd801a69b0e39bf923e97838134489ca4937fb7d081eccc0814b6811f71b3`; snippet SHA-256 `b1fb8269e3562204555fe961cfa7793c434c27db08b774418d9c952492427f96`; retained evidence `colon-source-window-3`.

```text
    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	const char *nsName = TclGetString(objv[2]);

	if ((*nsName != ':') || (*(nsName+1) != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

```

tcl9.0 9.0.4, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl9.0.4/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2603–2614. Full-source SHA-256 `ef5ef608fbb22c68d35317e09637f099380457e5d01ff8e1b67f2a8eeb70c110`; snippet SHA-256 `f27cf516ed3417377b63ba485291cb2120a5b97ecc644abb772f37af9f6a2334`; retained evidence `colon-source-window-4`.

```text
    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	const char *nsName = TclGetString(objv[2]);

	if ((nsName[0] != ':') || (nsName[1] != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

```

tcl9.1 9.1.0, revision `Retained release source; VCS revision not recorded. Exact complete file digest identifies the inspected bytes.`, `tmp/tcl9.1.0/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2503–2514. Full-source SHA-256 `2c5cf8968a3176aa8e5c316063594186f50109cecd2fbe3c3dff2f3035cefd1a`; snippet SHA-256 `b1fb8269e3562204555fe961cfa7793c434c27db08b774418d9c952492427f96`; retained evidence `colon-source-window-5`.

```text
    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	const char *nsName = TclGetString(objv[2]);

	if ((*nsName != ':') || (*(nsName+1) != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

```


## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::command_publication_slot`: Pure release-specific Procedure publication projection, including literal root colon handling; native CLI observations remain an independent finite question.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNamePurpose`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::lambda_namespace_input`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::lambda_namespace_path`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `literal_lambda_body`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::original_root_procedure_publication_preserves_a_literal_colon` (linked): Two original root spellings across five C policies produce their exact pure command slots. These Rust slot assertions do not replay the safe-child source or establish native object/holder correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::original_lambda_namespace_preserves_counted_object_and_lookup_purposes` (linked): Pure C8.5–C9.1 original getter-byte prefix and subsequent independent CString lookup projection retain count/zero/colon/Unicode boundaries. Unsupported C8.4/Jim C-specific purpose refuses. This is Rust API correspondence only, with no actual namespace existence, object/header identity, entered frame or native CLI replay.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `analyser::interp_visibility::tests::original_child_lambda_visibility_uses_its_own_literal_namespace` (linked): An authentic original literal lambda retains its independent C85+ namespace object prefix before selecting source lookup geometry. In the measured fixed :ns child script C85 procedure publication reparses its non-global holder into ns, so only the default-root source b.tcl has a conditional hidden visibility warning. C86+ keeps procedure publication in :ns while Apply selects ns, so both source a.tcl and default-root source b.tcl have conditional visibility hypotheses. Exact original head anchors remain asserted; successful operations, source-body entry and absence of observer mutation remain explicit assumptions. The procedure-publication CLI proof is independent of source-owner assertions and supplies no actual child frame/cell or selected callable.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_child_colon_publication/capture.py"
]
```

The retained runner uses original absolute SDK associations and writes provider folders; exact provider executables/environment and a fresh output directory are required. Source-model tests retain their own full-input premises and execution receipts. Tcl8.4 Apply and the tested Jim child surface remain unsupported.
