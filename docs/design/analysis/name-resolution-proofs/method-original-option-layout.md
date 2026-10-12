# naming.tcloo.method-original-option-layout

Kind: `native-observation`

## Problem statement

A method declaration can have a dash-prefixed name or formal list, so scanning the first dash word does not distinguish a method option from data. C9 additionally resolves an original export flag object by native CString index rules, making raw counted zero different from encoded C080. This matters when the VM and runtime select body/formal operands and visibility. The check is limited to the captured TclOO method argv and the explicit version-dependent forms.

## Question

Which original method argv forms and export option objects do C8.6 and C9 accept, and how do they distinguish public, unexported and true-private declarations?

## Conclusion

C8.6 accepts only the option-free name/args/body form and rejects the fourth operand. C9 selects a single option after the name only in the four-operand form; -private and its -p prefix, including a raw-zero suffix, select true-private visibility, while encoded C080 suffix and -- reject. -export is public and -unexport is protected; true-private is omitted from the -private method roster. Three-operand dashed formal lists and a method named -export remain data. C8.4/C8.5/Jim lack the measured OO command. These outputs do not prove cache, header, refcount, editable source or Rust implementation behavior.

## Scope

Both retained probe variants: public Tcl_NewStringObj(pointer,count)/Tcl_EvalObjv method option vectors, with independent ASCII NUL-terminated Tcl_Eval setup and query scripts; Jim runs only startup and OO-availability queries. Option bytes distinguish binary 00 from encoded C080. v1 dashed-formal script returns a literal $-arg; v2 uses ${-arg} and returns VALUE. No physical original-object window is inspected.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual executable SHA-256 88490db57f6ea1fbd36ee38b3de7de556d090e93bef8c88f924a40eecb6e965c; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual executable SHA-256 7c98303f2c0ff9d974c62a726c84563de34a996857a158c8e2c1244f68824d1e; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

The startup query reports this version and the actual OO command query is empty; no OO argv or private body is executed.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA-256 ae81ffd9b36fe978eb7f1a5314b295fe0fb1a3f843ef7e0dc42eeeab45bdd0a9; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

Every four-operand method form rejects with wrong-argument usage; option-free dashed formal/name controls succeed and external same-object Hidden rejects. The private worker controls reject as unavailable.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA-256 90317b32ab6c0da84b6daf0e7570399be4a0020b1d1d05b39edbc4a10dd00096; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

Original -private/-p/raw-zero option forms succeed, remain absent from the -private roster, reject external calls and return OK via the declaring method. Encoded C080 and -- reject; -export calls externally, -unexport only internally. The option-free data controls succeed.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA-256 176c6db17857a57e651b78c2872af7dd66daf673dd0e1d01c5c14007573d51db; public header, static library, Makefile, source owner, compile command and process/stream hashes retained independently for each variant.. Channel: Original counted Tcl objects passed to Tcl_EvalObjv plus ASCII NUL-terminated Tcl_Eval scripts.. Dialect: Tcl.

Original -private/-p/raw-zero option forms succeed, remain absent from the -private roster, reject external calls and return OK via the declaring method. Encoded C080 and -- reject; -export calls externally, -unexport only internally. The option-free data controls succeed.

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
- `tcl8.6-method-source` (source-anchor): [rust/tcl-vm/tests/data/native_method_visibility/source-anchors/8.6.18-method-definition.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/source-anchors/8.6.18-method-definition.json). SHA-256 `ae770a3f177f86babb8556fd58ab117c49d81a2fd36464a4b51d2e9269ab89bb`. JSON pointer `/snippet`. Actual pinned C source excerpt, full source digest and exact line coordinates; independent of native result observations.
- `tcl9.0-method-source` (source-anchor): [rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.0.4-method-definition.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.0.4-method-definition.json). SHA-256 `c2f92e10839e340409f7bb913d6e66dc907ede5dd5e7aaaab96e300a0cbd185f`. JSON pointer `/snippet`. Actual pinned C source excerpt, full source digest and exact line coordinates; independent of native result observations.
- `tcl9.1-method-source` (source-anchor): [rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.1.0-method-definition.json](../../../../rust/tcl-vm/tests/data/native_method_visibility/source-anchors/9.1.0-method-definition.json). SHA-256 `020c40130037ebebaa03b4a767721aa0835cbe6257b434bb289a5f4e9cff5021`. JSON pointer `/snippet`. Actual pinned C source excerpt, full source digest and exact line coordinates; independent of native result observations.

## Source inspection

tcl8.6 8.6.18, revision `Pinned Tcl 8.6.18 release source, retained full-file SHA-256`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 1572–1616. Full-source SHA-256 `56fb36d62f1ec51cca4cc5177bb90dc57986021206ea9700454ff67db7905e81`; snippet SHA-256 `149f61556901b38088f841f849e86b9e55a4b7d2730a1fa0ccd39871531f1ca4`; retained evidence `tcl8.6-method-source`.

```text
TclOODefineMethodObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceMethod = (clientData != NULL);
    Object *oPtr;
    int isPublic;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name args body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }
    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;

    /*
     * Create the method by using the right back-end API.
     */

    if (isInstanceMethod) {
	if (TclOONewProcInstanceMethod(interp, oPtr, isPublic, objv[1],
		objv[2], objv[3], NULL) == NULL) {
	    return TCL_ERROR;
	}
    } else {
	if (TclOONewProcMethod(interp, oPtr->classPtr, isPublic, objv[1],
		objv[2], objv[3], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Pinned Tcl 9.0.4 release source, retained full-file SHA-256`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 2309–2393. Full-source SHA-256 `1eb881c851f09719a9cb932c5ceb36034b96569affda2b2f870652a0bf29426d`; snippet SHA-256 `ca528ef582289d9f8483cc4767b04c7b0b97395acfa1a112abf3a591d5c9853f`; retained evidence `tcl9.0-method-source`.

```text
TclOODefineMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    /*
     * Table of export modes for methods and their corresponding enum.
     */

    static const char *const exportModes[] = {
	"-export",
	"-private",
	"-unexport",
	NULL
    };
    enum ExportMode {
	MODE_EXPORT,
	MODE_PRIVATE,
	MODE_UNEXPORT
    } exportMode;

    int isInstanceMethod = (clientData != NULL);
    Object *oPtr;
    int isPublic = 0;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?option? args body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }
    if (objc == 5) {
	if (Tcl_GetIndexFromObj(interp, objv[2], exportModes, "export flag",
		0, &exportMode) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (exportMode) {
	case MODE_EXPORT:
	    isPublic = PUBLIC_METHOD;
	    break;
	case MODE_PRIVATE:
	    isPublic = TRUE_PRIVATE_METHOD;
	    break;
	case MODE_UNEXPORT:
	    isPublic = 0;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    } else {
	if (IsPrivateDefine(interp)) {
	    isPublic = TRUE_PRIVATE_METHOD;
	} else {
	    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
		    ? PUBLIC_METHOD : 0;
	}
    }

    /*
     * Create the method by using the right back-end API.
     */

    if (isInstanceMethod) {
	if (TclOONewProcInstanceMethod(interp, oPtr, isPublic, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    } else {
	if (TclOONewProcMethod(interp, oPtr->classPtr, isPublic, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Pinned Tcl 9.1.0 release source, retained full-file SHA-256`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 2271–2352. Full-source SHA-256 `bd254581a59362769c56269dfb009dd9ed5e5b9bcf00ac67b5738e97e50b5106`; snippet SHA-256 `a10f879e9ab4d7269c1f889ab63cb99cf586d0d07f8f5423c4b69c4d36c0e57b`; retained evidence `tcl9.1-method-source`.

```text
TclOODefineMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    /*
     * Table of export modes for methods and their corresponding enum.
     */

    static const char *const exportModes[] = {
	"-export",
	"-private",
	"-unexport",
	NULL
    };
    enum ExportMode {
	MODE_EXPORT,
	MODE_PRIVATE,
	MODE_UNEXPORT
    } exportMode;

    bool isInstanceMethod = (clientData != NULL);
    Object *oPtr;
    int flags = 0;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?option? args body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceMethod && !oPtr->classPtr) {
	return ReportAbuse(interp);
    }
    if (objc == 5) {
	if (Tcl_GetIndexFromObj(interp, objv[2], exportModes, "export flag",
		0, &exportMode) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (exportMode) {
	case MODE_EXPORT:
	    flags = PUBLIC_METHOD;
	    break;
	case MODE_PRIVATE:
	    flags = TRUE_PRIVATE_METHOD;
	    break;
	case MODE_UNEXPORT:
	    flags = 0;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    } else {
	if (IsPrivateDefine(interp)) {
	    flags = TRUE_PRIVATE_METHOD;
	} else {
	    flags = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
		    ? PUBLIC_METHOD : 0;
	}
    }

    /*
     * Create the method by using the right back-end API.
     */

    if (isInstanceMethod) {
	if (TclOONewProcInstanceMethod(interp, oPtr, flags, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    } else {
	if (TclOONewProcMethod(interp, oPtr->classPtr, flags, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```


## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `NativeTclooMethodDefinitionProtocol::layout`: Selected pure method argv/visibility semantics, independent of body or native table capability.
- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `def_method`: Consume original option objects and current method flags, or authentic caller-provider scope without a context-dependent cache grant.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `def_method`: Consume the same selected declaration/caller boundary on the runtime port.
- [rust/tcl-vm/src/cmd_oo/native_private_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::method_option_layout_and_private_scope_match_original_native_controls` (linked): Compare selected C8.6/C9 original method option definition completion code/bytes and successful/internal/external outputs. Private rosters compare the exact one-element invoke result. Class caller controls compare exact results; subclass scope rejection compares completion code only, leaving bootstrap diagnostic-roster ordering independent. No pass is inferred from the binding.
- [runtime/rust/src/cmd_oo/native_private_tests.rs](../../../../runtime/rust/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::method_option_layout_and_private_scope_match_original_native_controls` (linked): Same finite original option/caller comparisons on independently selected runtime cores; no fresh native or Rust execution is implied.
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `native_tcloo_method_definition::tests::method_argc_and_visibility_keep_private_separate_from_unexported` (linked): Selected release-dependent argc, option ordinal and distinct visibility recipe; a pure recipe test grants no native registration or physical method table.

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
