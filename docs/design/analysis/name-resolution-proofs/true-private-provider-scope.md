# naming.tcloo.true-private-provider-scope

Kind: `native-observation`

## Problem statement

Treating true-private as ordinary unexported visibility lets subclasses call a declaring class private method and incorrectly permits external same-object access to ordinary protected methods. Conversely, restricting every private call to the same object rejects authentic class-scoped access to another instance. This check distinguishes these cases in a single-inheritance class graph and includes a private forward and export conversion; it does not establish arbitrary filter, mixin, object-private or cache behavior.

## Question

For the captured C9 class private methods and forward, which original caller provider can invoke them, and does external same-object access make an ordinary unexported method callable?

## Conclusion

C9 allows an inherited method executing with its original declaring-class scope to invoke that class private method, including external [self] and another same-class object. A subclass-declared caller cannot invoke it. The declaring private takes priority over a public subclass override; a private forward has the same hidden roster/external-call boundary. Export converts the private method to public. External [self] access to a merely unexported Hidden method rejects on C8.6 and C9. Earlier OO/private-unavailable answers remain explicit. No general MRO, object-private, native method-cache or Rust correctness claim is issued.

## Scope

Actual ASCII NUL-terminated Tcl_Eval scripts with fresh interpreters and named single-inheritance classes. C9 class-scoped private calls, private forward, export conversion, inherited/subclass caller and public override controls. C8.6 independently rejects the private worker and rejects external same-object unexported access. Jim and C8.4/C8.5 only report absent OO. v1 and v2 source/outcomes remain separate.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual executable SHA-256 88490db57f6ea1fbd36ee38b3de7de556d090e93bef8c88f924a40eecb6e965c; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual executable SHA-256 7c98303f2c0ff9d974c62a726c84563de34a996857a158c8e2c1244f68824d1e; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### tcl8.6

Status: `unsupported`. Version: 8.6.18. Build: Actual executable SHA-256 ae81ffd9b36fe978eb7f1a5314b295fe0fb1a3f843ef7e0dc42eeeab45bdd0a9; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

Every four-operand method form rejects with wrong-argument usage; option-free dashed formal/name controls succeed and external same-object Hidden rejects. The private worker controls reject as unavailable.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA-256 90317b32ab6c0da84b6daf0e7570399be4a0020b1d1d05b39edbc4a10dd00096; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

INHERITED_OWN_PRIVATE, EXTERNAL_SELF_PRIVATE, CROSS_OBJECT_CLASS_PRIVATE and DECLARER_PRIVATE_BEFORE_OVERRIDE return PRIVATE; SUBCLASS_PRIVATE_MISS errors. PRIVATE_FORWARD is hidden and callable in scope, EXPORT_CONVERTS_PRIVATE returns PRIVATE externally. EXTERNAL_SELF_UNEXPORTED errors.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA-256 176c6db17857a57e651b78c2872af7dd66daf673dd0e1d01c5c14007573d51db; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

INHERITED_OWN_PRIVATE, EXTERNAL_SELF_PRIVATE, CROSS_OBJECT_CLASS_PRIVATE and DECLARER_PRIVATE_BEFORE_OVERRIDE return PRIVATE; SUBCLASS_PRIVATE_MISS errors. PRIVATE_FORWARD is hidden and callable in scope, EXPORT_CONVERTS_PRIVATE returns PRIVATE externally. EXTERNAL_SELF_UNEXPORTED errors.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA-256 74716add74e406783e11369cf71511af5570ce9b0a3acff56f7ad8057764bde6; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: ASCII NUL-terminated startup and OO-availability source passed to Jim_Eval; no Jim OO operand vector executes.. Dialect: Jim Tcl.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this exact method argv or private caller question.

## Exact evidence

- `v1-input` (input): [rust/tcl-vm/tests/data/native_method_visibility/v1/probe.c](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/probe.c). SHA-256 `58a2cd9b52855324f4f4cec366feac1d323985353516506631c19441cc942e66`. Exact original probe or complete six-provider receipt; earlier variant is preserved independently.
- `v1-aggregate` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/receipt.json). SHA-256 `75f87d9ff1c80f8a64249bea414e2fde0a81defd84778d35fdce194f106810b2`. Exact original probe or complete six-provider receipt; earlier variant is preserved independently.
- `v2-input` (input): [rust/tcl-vm/tests/data/native_method_visibility/v2/probe.c](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/probe.c). SHA-256 `1d04bb72a4cbd6b28aba5e6b4692dadbade56c409e4abf4591544467c1d636f7`. Exact original probe or complete six-provider receipt; earlier variant is preserved independently.
- `v2-aggregate` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/receipt.json). SHA-256 `31e9c39253758f93788526475a69cc699eecf8f3a488df1b1f72fbc083dfe170`. Exact original probe or complete six-provider receipt; earlier variant is preserved independently.
- `v1-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/receipt.json). SHA-256 `2564b18b553c09b027133d800e467a6c68202f935a4f9dd40c3b28b383287109`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/stdout.tsv). SHA-256 `46f05ae01d71cf78a58b0557e6cab7ddc56fe225a9872ef14387c1c9227f2fc3`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/receipt.json). SHA-256 `39494acf9f91f2930e780bb48bc55a5f22623542313f19305c408ca3df73115a`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/stdout.tsv). SHA-256 `46f05ae01d71cf78a58b0557e6cab7ddc56fe225a9872ef14387c1c9227f2fc3`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/receipt.json). SHA-256 `1aa2268a313e86e0604ef643119bc204ed0c58835e6e0c45070b93cc6d93c394`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/stdout.tsv). SHA-256 `6763de318120d961bc97b57f9f5476dfdd1a9bc433a782898d4c618ca28564d0`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/receipt.json). SHA-256 `5b6a7ae93722cde4388803ea2a49d3d5931a5c2b7e561a1041b0a3cd6d6c835b`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/stdout.tsv). SHA-256 `6763de318120d961bc97b57f9f5476dfdd1a9bc433a782898d4c618ca28564d0`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/receipt.json). SHA-256 `83c563b89b85904dd4142770027e36278d99327791afd6cdde0cb2cda0f44a90`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/stdout.tsv). SHA-256 `2f62b1d6a24dda595d4b7f82cc3b39ebed1df928ed954b3af616ac033fdfa150`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/receipt.json). SHA-256 `9e63de16c5e4aeae86b9179b6f6c969a23fb1691cefd2db04efc8594bc6edeff`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/stdout.tsv). SHA-256 `41fa10ac8d3a9cad6b698637550a0bd116d4dbf934dfcc48da7d41bb88de9cba`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/receipt.json). SHA-256 `60099ee142fa49002eaf9eacd1a285eeb43ade5d393c0ea3b8ebd672859f2233`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/stdout.tsv). SHA-256 `3d9ec3d2121a4506b1f122761ac3b7652711abb22985c2d1f08dc554f2d3df8b`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/receipt.json). SHA-256 `c714efbb51993cff45d59d5192ae2eecd82e338cf623d08a863b270d1cdeb695`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/stdout.tsv). SHA-256 `622f41fb9ea1f632f0e0aaae1312f92ea052a4a1e8dd8080b8bc86ee5d73e5c3`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/receipt.json). SHA-256 `33111cc9ec24bbe31135144d22c5153d78571c835870b9456cf08b756b7d0e97`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/stdout.tsv). SHA-256 `34d9b8da3171d8326a92c259fd919d0c8505ed91201abdecfe07ec6935d7b76e`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/receipt.json). SHA-256 `ae50c5c55fcc4aca2d770539d52578e0f3d6bde027742189d157964cddee6a07`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/stdout.tsv). SHA-256 `a83a670578346a6c7364f6e95639fb25cef5624ed52cb2365b432bc6faa48408`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v1/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/jim/receipt.json). SHA-256 `2d4c6f206edf77d8ccb57b0ad784fe4d8ff2a4ce8214fc016c7b317b3c4b407f`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/jim/stdout.tsv). SHA-256 `5707d1b91c4f468cfe79a38218291016830110fb0bc37c8d942c2c7eff73518a`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v1-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v1/jim/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v1/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_method_visibility/v2/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/jim/receipt.json). SHA-256 `6ad9a582711d4fbfac2ab06ab6d91069e235a5edbc0a58b5aedf2eb0a7354a0d`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/jim/stdout.tsv). SHA-256 `5707d1b91c4f468cfe79a38218291016830110fb0bc37c8d942c2c7eff73518a`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `v2-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_method_visibility/v2/jim/stderr](../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider/process/output correspondence. Guest errors are recorded rows; all harness processes exited zero.
- `tcl9.0-scope-source` (source-anchor): [rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.0.4-private-scope.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.0.4-private-scope.json). SHA-256 `dcaab40eba28bb9997244bda82f480017410dcd7f57ebe2c96f684514247ab8b`. JSON pointer `/snippet`. Actual pinned C source excerpt, full source digest and exact line coordinates; independent of native result observations.
- `tcl9.1-scope-source` (source-anchor): [rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.1.0-private-scope.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.1.0-private-scope.json). SHA-256 `449550606c36277bbd62993b05adb7464fb7a38411823bad77ec0f5b61a8e559`. JSON pointer `/snippet`. Actual pinned C source excerpt, full source digest and exact line coordinates; independent of native result observations.

## Source inspection

tcl9.0 9.0.4, revision `Pinned Tcl 9.0.4 release source, retained full-file SHA-256`, `generic/tclOOCall.c`, function `AddInstancePrivateToCallContext / AddSimpleChainToCallContext`, lines 839–950. Full-source SHA-256 `d28f9c8a4e5ff6382994f050fcdd1e0aa2d6f79b184ac966bc54122f610cc075`; snippet SHA-256 `f5dd4830571febdbf8a60948c30af21aa0830327b12e80763b6e5323225e272f`; retained evidence `tcl9.0-scope-source`.

```text
AddInstancePrivateToCallContext(
    Object *const oPtr,		/* Object to add call chain entries for. */
    Tcl_Obj *const methodName,	/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    int flags)			/* What sort of call chain are we building. */
{
    Tcl_HashEntry *hPtr;
    Method *mPtr;
    int donePrivate = 0;

    if (oPtr->methodsPtr) {
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodName);
	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (IS_PRIVATE(mPtr)) {
		AddMethodToCallChain(mPtr, cbPtr, NULL, NULL, flags);
		donePrivate = 1;
	    }
	}
    }
    return donePrivate;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddSimpleChainToCallContext --
 *
 *	The core of the call-chain construction engine, this handles calling a
 *	particular method on a particular object. Note that filters and
 *	unknown handling are already handled by the logic that uses this
 *	function. Returns true if a private method was one of those found.
 *
 * ----------------------------------------------------------------------
 */

static inline int
AddSimpleChainToCallContext(
    Object *const oPtr,		/* Object to add call chain entries for. */
    Class *const contextCls,	/* Context class; the currently considered
				 * class is equal to this, private methods may
				 * also be added. [TIP 500] */
    Tcl_Obj *const methodNameObj,
				/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what call chain entries
				 * have been processed. */
    int flags,			/* What sort of call chain are we building. */
    Class *const filterDecl)	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
{
    Tcl_Size i;
    int foundPrivate = 0, blockedUnexported = 0;
    Tcl_HashEntry *hPtr;
    Method *mPtr;

    if (!(flags & (KNOWN_STATE | SPECIAL)) && oPtr->methodsPtr) {
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);

	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (!IS_PRIVATE(mPtr)) {
		if (WANT_PUBLIC(flags)) {
		    if (!IS_PUBLIC(mPtr)) {
			blockedUnexported = 1;
		    } else {
			flags |= DEFINITE_PUBLIC;
		    }
		} else {
		    flags |= DEFINITE_PROTECTED;
		}
	    }
	}
    }
    if (!(flags & SPECIAL)) {
	Class *mixinPtr;

	FOREACH(mixinPtr, oPtr->mixins) {
	    if (contextCls) {
		foundPrivate |= AddPrivatesFromClassChainToCallContext(
			mixinPtr, contextCls, methodNameObj, cbPtr,
			doneFilters, flags|TRAVERSED_MIXIN, filterDecl);
	    }
	    foundPrivate |= AddSimpleClassChainToCallContext(mixinPtr,
		    methodNameObj, cbPtr, doneFilters,
		    flags | TRAVERSED_MIXIN, filterDecl);
	}
	if (oPtr->methodsPtr && !blockedUnexported) {
	    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);
	    if (hPtr != NULL) {
		mPtr = (Method *) Tcl_GetHashValue(hPtr);
		if (!IS_PRIVATE(mPtr)) {
		    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			    flags);
		}
	    }
	}
    }
    if (!oPtr->selfCls) {
	return foundPrivate;
    }
    if (contextCls) {
	foundPrivate |= AddPrivatesFromClassChainToCallContext(oPtr->selfCls,
		contextCls, methodNameObj, cbPtr, doneFilters, flags,
		filterDecl);
    }
    if (!blockedUnexported) {

```

tcl9.1 9.1.0, revision `Pinned Tcl 9.1.0 release source, retained full-file SHA-256`, `generic/tclOOCall.c`, function `AddInstancePrivateToCallContext / AddSimpleChainToCallContext`, lines 860–969. Full-source SHA-256 `8fc03042b0551bd87b9af833f52cdc0819c7c2b3f23c9982895f06128bfaf4d0`; snippet SHA-256 `ac5a0e6368813a28c24dff3e2a927a0686fabc9223f462ba24d9b8e0416104d0`; retained evidence `tcl9.1-scope-source`.

```text
AddInstancePrivateToCallContext(
    Object *const oPtr,		/* Object to add call chain entries for. */
    Tcl_Obj *const methodName,	/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    int flags)			/* What sort of call chain are we building. */
{
    Tcl_HashEntry *hPtr;
    Method *mPtr;
    bool donePrivate = false;

    if (oPtr->methodsPtr) {
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodName);
	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (IS_PRIVATE(mPtr)) {
		AddMethodToCallChain(mPtr, cbPtr, NULL, NULL, flags);
		donePrivate = true;
	    }
	}
    }
    return donePrivate;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddSimpleChainToCallContext --
 *
 *	The core of the call-chain construction engine, this handles calling a
 *	particular method on a particular object. Note that filters and
 *	unknown handling are already handled by the logic that uses this
 *	function. Returns true if a private method was one of those found.
 *
 * ----------------------------------------------------------------------
 */
static inline bool
AddSimpleChainToCallContext(
    Object *const oPtr,		/* Object to add call chain entries for. */
    Class *const contextCls,	/* Context class; the currently considered
				 * class is equal to this, private methods may
				 * also be added. [TIP 500] */
    Tcl_Obj *const methodNameObj,
				/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what call chain entries
				 * have been processed. */
    int flags,			/* What sort of call chain are we building. */
    Class *const filterDecl)	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
{
    Tcl_Size i;
    bool foundPrivate = false, blockedUnexported = false;
    Tcl_HashEntry *hPtr;
    Method *mPtr;

    if (!(flags & (KNOWN_STATE | SPECIAL)) && oPtr->methodsPtr) {
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);

	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (!IS_PRIVATE(mPtr)) {
		if (WANT_PUBLIC(flags)) {
		    if (!IS_PUBLIC(mPtr)) {
			blockedUnexported = true;
		    } else {
			flags |= DEFINITE_PUBLIC;
		    }
		} else {
		    flags |= DEFINITE_PROTECTED;
		}
	    }
	}
    }
    if (!(flags & SPECIAL)) {
	Class *mixinPtr;

	FOREACH(mixinPtr, oPtr->mixins) {
	    if (contextCls) {
		foundPrivate |= AddPrivatesFromClassChainToCallContext(
			mixinPtr, contextCls, methodNameObj, cbPtr,
			doneFilters, flags|TRAVERSED_MIXIN, filterDecl);
	    }
	    foundPrivate |= AddSimpleClassChainToCallContext(mixinPtr,
		    methodNameObj, cbPtr, doneFilters,
		    flags | TRAVERSED_MIXIN, filterDecl);
	}
	if (oPtr->methodsPtr && !blockedUnexported) {
	    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);
	    if (hPtr != NULL) {
		mPtr = (Method *) Tcl_GetHashValue(hPtr);
		if (!IS_PRIVATE(mPtr)) {
		    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			    flags);
		}
	    }
	}
    }
    if (!oPtr->selfCls) {
	return foundPrivate;
    }
    if (contextCls) {
	foundPrivate |= AddPrivatesFromClassChainToCallContext(oPtr->selfCls,
		contextCls, methodNameObj, cbPtr, doneFilters, flags,
		filterDecl);
    }

```


## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `NativeTclooMethodDefinitionProtocol::default_visibility`: Selected pure method argv/visibility semantics, independent of body or native table capability.
- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `oo_invoke_value_with_head`: Consume original option objects and current method flags, or authentic caller-provider scope without a context-dependent cache grant.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `Interp::oo_invoke_with_original`: Consume the same selected declaration/caller boundary on the runtime port.
- [rust/tcl-vm/src/cmd_oo/native_private_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::method_option_layout_and_private_scope_match_original_native_controls` (linked): Compare selected C8.6/C9 original method option definition completion code/bytes and successful/internal/external outputs. Private rosters compare the exact one-element invoke result. Class caller controls compare exact results; subclass scope rejection compares completion code only, leaving bootstrap diagnostic-roster ordering independent. No pass is inferred from the binding.
- [runtime/rust/src/cmd_oo/native_private_tests.rs](../../../../runtime/rust/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::method_option_layout_and_private_scope_match_original_native_controls` (linked): Same finite original option/caller comparisons on independently selected runtime cores; no fresh native or Rust execution is implied.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-vm/tests/data/native_method_visibility/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-method-visibility-reconfirmation"
]
```

Requires exact retained probe/public-header/static-library/Makefile/source-owner hashes and configured matching providers. Recompile both variants and compare full process exit/stdout/stderr; guest errors are expected rows. --verify-only checks inputs/streams without a native launch. No cache/header/refcount, arbitrary observer or Rust execution result is asserted.
