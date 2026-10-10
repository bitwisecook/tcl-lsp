# naming.diagnostics.original-error-code-metadata-not-message

Kind: `native-observation`

## Problem statement

Does guest error text that looks like a wrong-arguments diagnostic select its error-code classification, or does classification belong to the original error producer and explicit metadata? Distinguish an arbitrary error message, genuine set wrongargs and explicit supplied error code independently in every original provider.

## Question

Does wrong-arguments-looking error text establish public errorCode metadata, and how do actual set arity errors and explicit error-code arguments differ across the pinned original C releases and Jim?

## Conclusion

All six original providers replace the explicit seed with NONE for an arbitrary error whose message resembles a wrong-arguments diagnostic. Genuine catch {set} reports NONE in C8.4.20, C8.5.19 and current Jim 0.84-9-g5bac7c9, but TCL WRONGARGS in C8.6.18, C9.0.4 and C9.1.0. The five C releases accept the explicit error third argument and retain USER CODE while reporting the same arbitrary message. Jim rejects that three-argument command with its own error-message arity diagnostic and reports NONE; no Jim explicit-code positive is observed. Every public errorCode query succeeds with availability1, including Jim, and all case catch codes are1. Thus matching message text does not establish emitted diagnostic metadata, and the actual wrong-arguments producer has a measured release boundary. Six fresh original CLI processes exit0 with empty stderr and complete three-case markers. No generic error/completion classification, private object/cache/frame/worker identity, source edit, handler admission, BIG-IP transfer or Rust assertion result is established.

A linked software embedding control keeps original caller-owned error-code/error-info metadata independently of arbitrary message text. This is a transport contract distinct from the six original Native CLI processes and their measured release boundary.

The selected software wrong-arguments protocol compares its error-code bytes with the six original published genuine catch-set rows. Those rows independently establish NONE for C8.4/C8.5/Jim and TCL WRONGARGS for C8.6/C9.0/C9.1; arbitrary message text remains separate.

## Scope

The immutable original ASCII LF source performs three caught public script operations in one fresh process per provider, setting a distinct seed before each. Count-preserving binary-scan hex retains exact result and public errorCode bytes; both availability and returned value are measured. Seeds are overwritten in every captured case. The fixed source does not contain raw NUL, non-ASCII, binary-produced keys, private headers or direct object-vector entry. Fresh info patchlevel queries are retained independently of prior version receipts reused under exact original CLI/source/header/configuration/static archive pins. No new compile is run; selected original CLI executable identities, launch argv, explicit TCL_LIBRARY overlay and prelaunch 162 required pins are retained. Inherited environment, compiler version and unrelated system library identities are unrecorded. Nineteen exact original Tcl_ErrorObjCmd/Tcl_SetObjCmd/Tcl_WrongNumArgs and Jim error/set/wrongargs/registration windows explain producers separately from observed output. The unchanged preparation request includes a conditional unsupported/untouched Jim errorCode caution; the actual captured Jim query instead succeeds and reports NONE. Only Jim third-argument explicit-code acceptance is purpose-unavailable; it is never used to label the supported public query N/A. These three cases do not certify other commands, release points, arbitrary catch options or message parsing. Software API controls and typed Guest/Host settlement require independent receipts; all prior native records remain unchanged. BIG-IP is not tested.

The embedding control has no executed outcome attached. Original Native430 provider answers, public query availability, explicit-code acceptance/refusal, exact source/streams and evidence pins remain unchanged. Software transport does not confer a generic diagnostic class, Native object/header/frame or successful handler.

The linked assertion definition has no executed result attached by this binding. Its NativeOrigin and String-producer assertions are software selection contracts, not original public header observations. Original Native430 provider answers, explicit-code acceptance/refusal, evidence/source windows and replay remain unchanged.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Exact original CLI SHA f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; original static archive /workspace/tcl-lsp/tmp/tcl8.4.20/unix/libtcl8.4.a SHA 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47; existing build inputs and version receipt SHA 0f7234e7d052215ad3586d3f98dbfe640c84868c10d915cda35c612b11dfcf63 are retained, not recompiled. Fresh patchlevel query agrees. Compiler and inherited environment versions unrecorded.. Channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case. Dialect: Actual pinned original provider release and distribution; Jim explicit error-code argument acceptance unavailable, supported metadata query separate.

Arbitrary-message NONE; genuine set wrongargs NONE; explicit USER CODE accepted with the unchanged arbitrary message.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact original CLI SHA e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; original static archive /workspace/tcl-lsp/tmp/tcl8.5.19/unix/libtcl8.5.a SHA 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc; existing build inputs and version receipt SHA 5c357fa1bd8f44e02b0b08fca76640fecbe85297493b116cb8e8e0a78aa617e7 are retained, not recompiled. Fresh patchlevel query agrees. Compiler and inherited environment versions unrecorded.. Channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case. Dialect: Actual pinned original provider release and distribution; Jim explicit error-code argument acceptance unavailable, supported metadata query separate.

Arbitrary-message NONE; genuine set wrongargs NONE; explicit USER CODE accepted with the unchanged arbitrary message.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact original CLI SHA 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; original static archive /workspace/tcl-lsp/tmp/tcl8.6.18/unix/libtcl8.6.a SHA 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb; existing build inputs and version receipt SHA 9a89b544c51fe11d8dc302119e7047e0cdc0dceb183572c743c430a88c155af1 are retained, not recompiled. Fresh patchlevel query agrees. Compiler and inherited environment versions unrecorded.. Channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case. Dialect: Actual pinned original provider release and distribution; Jim explicit error-code argument acceptance unavailable, supported metadata query separate.

Arbitrary-message NONE; genuine set wrongargs TCL WRONGARGS; explicit USER CODE accepted with the unchanged arbitrary message.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact original CLI SHA f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; original static archive /workspace/tcl-lsp/tmp/tcl9.0.4/unix/libtcl9.0.a SHA dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4; existing build inputs and version receipt SHA adcdc0557f5279db1a62ff25ff750400f64280c9c6d0151614ed663aa8fe1fc3 are retained, not recompiled. Fresh patchlevel query agrees. Compiler and inherited environment versions unrecorded.. Channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case. Dialect: Actual pinned original provider release and distribution; Jim explicit error-code argument acceptance unavailable, supported metadata query separate.

Arbitrary-message NONE; genuine set wrongargs TCL WRONGARGS; explicit USER CODE accepted with the unchanged arbitrary message.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact original CLI SHA 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; original static archive /workspace/tcl-lsp/tmp/tcl9.1.0/unix/libtcl9.1.a SHA 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db; existing build inputs and version receipt SHA 496e47fce5ab0c75d2c56654c7b242652d257dfd2f049869934e5190c6671bb8 are retained, not recompiled. Fresh patchlevel query agrees. Compiler and inherited environment versions unrecorded.. Channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case. Dialect: Actual pinned original provider release and distribution; Jim explicit error-code argument acceptance unavailable, supported metadata query separate.

Arbitrary-message NONE; genuine set wrongargs TCL WRONGARGS; explicit USER CODE accepted with the unchanged arbitrary message.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Exact original CLI SHA e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; original static archive /workspace/.proofs/native-providers/jimtcl/libjim.a SHA a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da; existing build inputs and version receipt SHA 006a4501536c0ae6e1919ef33f8cfcad38ebd8698b965f90abfcd6d364e32b54 are retained, not recompiled. Fresh patchlevel query agrees. Compiler and inherited environment versions unrecorded.. Channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case. Dialect: Actual pinned original provider release and distribution; Jim explicit error-code argument acceptance unavailable, supported metadata query separate.

Arbitrary-message NONE; genuine set wrongargs NONE; explicit third argument rejected by error arity, with NONE. The public errorCode query succeeds; only explicit-code acceptance is unavailable.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Original error producer and public diagnostic metadata purpose.

No BIG-IP process, diagnostic code, completion or transfer observation is executed by this request.

## Exact evidence

- `native_error_code_metadata_original-closure.json` (provider): [rust/tcl-registry/tests/data/native_error_code_metadata_original/closure.json](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/closure.json). SHA-256 `babb7238dcc4f632ab95e00c47f241e64f5b468bdcb556461626a7526880b33c`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-launch.py` (input): [rust/tcl-registry/tests/data/native_error_code_metadata_original/launch.py](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/launch.py). SHA-256 `98ef4a56ddbca497f9b2838c10ba5e457997dff98052cdde96024090c999fa4d`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-prepare.py` (input): [rust/tcl-registry/tests/data/native_error_code_metadata_original/prepare.py](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/prepare.py). SHA-256 `9c56eaa97574533b4a9a9f1a5227e88285d3edba435a64e417733de82f8d320f`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-probe.tcl` (input): [rust/tcl-registry/tests/data/native_error_code_metadata_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/probe.tcl). SHA-256 `11c9c5def98a57ede58c2814f742488628982fecf4e42c60967db0bf5992f6f2`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-8.4.20-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.4.20/execute.stderr](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.4.20/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-8.4.20-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.4.20/execute.stdout](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.4.20/execute.stdout). SHA-256 `dd909149d2f5534a455124ec9a7491b5bca6ff5a082d9b99551c29ed4590f27a`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.4.20/receipt.json). SHA-256 `d303abca99c04ae155f49bb54d02904f623a6fb4a7d6bdfc28a5f666f94b8bc6`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-8.5.19-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.5.19/execute.stderr](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.5.19/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-8.5.19-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.5.19/execute.stdout](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.5.19/execute.stdout). SHA-256 `d0d76f0dbfc08bd25483d69fc7f0679cbecb6d570169247a34a0341facfcf925`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.5.19/receipt.json). SHA-256 `1f4024a0ae9711df97d8fe17de8a1f43b88e4ba60c3db7e99db4aa3a0f53a7bc`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-8.6.18-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.6.18/execute.stderr](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.6.18/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-8.6.18-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.6.18/execute.stdout](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.6.18/execute.stdout). SHA-256 `ebf35bec5737568b32b72bccf46ac4ce6a65d6efd6bf9cf92b5a07c76dce4da8`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/8.6.18/receipt.json). SHA-256 `7e54be4a0128299aa90ded292cc6afe5525c40542c99ab2dc5c4faf8d26a81b5`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-9.0.4-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.0.4/execute.stderr](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.0.4/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-9.0.4-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.0.4/execute.stdout](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.0.4/execute.stdout). SHA-256 `1c004ea0f1110c4f0f8dfe933c53d03556529e8fdbef0bcb222085cd0186ce5a`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.0.4/receipt.json). SHA-256 `74755427050c488fb8aaa479661559cb61ba24510857eadcc19a605267d625da`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-9.1.0-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.1.0/execute.stderr](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.1.0/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-9.1.0-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.1.0/execute.stdout](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.1.0/execute.stdout). SHA-256 `833cd154081bdfd6c36e6705622eb13f14482d5e7bed146826418fcf97622cbd`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/9.1.0/receipt.json). SHA-256 `f83f33926723d59876da4f5230d7fe67c9f7aab2992ed4830dfd858068db0789`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-jim-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/jim/execute.stderr](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/jim/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-jim-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/jim/execute.stdout](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/jim/execute.stdout). SHA-256 `3015323e25f6a9d5a6d40fb307f7970f6f100baf5d761d45461f220817df3789`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-providers-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/providers/jim/receipt.json). SHA-256 `7fe8f51c63dfc7993488c67e05c07c99e47d8113ec54b24e116f2083c547a637`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-request.json` (provider): [rust/tcl-registry/tests/data/native_error_code_metadata_original/request.json](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/request.json). SHA-256 `32f27a866072782f27faedb50f0b783b339846d5c06c5e82cdf6b8a34fa14a17`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-8.4.20-tclCmdAH.c.Tcl_ErrorObjCmd.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.4.20/tclCmdAH.c.Tcl_ErrorObjCmd.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.4.20/tclCmdAH.c.Tcl_ErrorObjCmd.txt). SHA-256 `f9cde389a031a9c7ac2e8fc579448b08cbcd7061bcf60f2597d60c8e9c43d947`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-8.4.20-tclIndexObj.c.Tcl_WrongNumArgs.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.4.20/tclIndexObj.c.Tcl_WrongNumArgs.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.4.20/tclIndexObj.c.Tcl_WrongNumArgs.txt). SHA-256 `8125161d96c388433cfd68cc8b7c730e7a2b3132c8ba6d1ebe2e3320aeb5f053`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-8.4.20-tclVar.c.Tcl_SetObjCmd.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.4.20/tclVar.c.Tcl_SetObjCmd.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.4.20/tclVar.c.Tcl_SetObjCmd.txt). SHA-256 `95c053949276a7d26a3bf7745298321b570cbc28a5529e4e196473e12cc7acb8`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-8.5.19-tclCmdAH.c.Tcl_ErrorObjCmd.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.5.19/tclCmdAH.c.Tcl_ErrorObjCmd.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.5.19/tclCmdAH.c.Tcl_ErrorObjCmd.txt). SHA-256 `9b02909051ff9f8faba2d3adc092620f73e028fcd8873fa2180d3878b154d908`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-8.5.19-tclIndexObj.c.Tcl_WrongNumArgs.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.5.19/tclIndexObj.c.Tcl_WrongNumArgs.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.5.19/tclIndexObj.c.Tcl_WrongNumArgs.txt). SHA-256 `1c0f94ce78433aee6e11452dc3a5eba65134a7d9602efcacaa0e49f748376b95`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-8.5.19-tclVar.c.Tcl_SetObjCmd.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.5.19/tclVar.c.Tcl_SetObjCmd.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.5.19/tclVar.c.Tcl_SetObjCmd.txt). SHA-256 `375fd43683b9ebbe7921c2e2814068cdec1e9698cc008648fb8de7fdd31e3a49`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-8.6.18-tclCmdAH.c.Tcl_ErrorObjCmd.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.6.18/tclCmdAH.c.Tcl_ErrorObjCmd.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.6.18/tclCmdAH.c.Tcl_ErrorObjCmd.txt). SHA-256 `a19206fb4486172c4aa646ce80515e42b1963c81f9890793e065ba40b21dcffa`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-8.6.18-tclIndexObj.c.Tcl_WrongNumArgs.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.6.18/tclIndexObj.c.Tcl_WrongNumArgs.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.6.18/tclIndexObj.c.Tcl_WrongNumArgs.txt). SHA-256 `6521cc4830fab0f2cc0cf206b252858fb1298a31c4a9acd4e5ade7d045d77203`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-8.6.18-tclVar.c.Tcl_SetObjCmd.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.6.18/tclVar.c.Tcl_SetObjCmd.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/8.6.18/tclVar.c.Tcl_SetObjCmd.txt). SHA-256 `6426a93f26b10292570b9da50721fe64949f5498290b705ed76293ac9f92e105`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-9.0.4-tclCmdAH.c.Tcl_ErrorObjCmd.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.0.4/tclCmdAH.c.Tcl_ErrorObjCmd.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.0.4/tclCmdAH.c.Tcl_ErrorObjCmd.txt). SHA-256 `3aa71c1110953d8b9c577f83aa86f6d0da07dec66e70ebaa0b7111a58b9dbeb1`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-9.0.4-tclIndexObj.c.Tcl_WrongNumArgs.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.0.4/tclIndexObj.c.Tcl_WrongNumArgs.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.0.4/tclIndexObj.c.Tcl_WrongNumArgs.txt). SHA-256 `77260f12eea3e8630d845887d77c26c2cbe74a04c085ad677caed22400698223`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-9.0.4-tclVar.c.Tcl_SetObjCmd.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.0.4/tclVar.c.Tcl_SetObjCmd.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.0.4/tclVar.c.Tcl_SetObjCmd.txt). SHA-256 `237f5aa94832d4839fcbcdbe9436c8a8b9ef15217649019293cbf984b8601fdd`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-9.1.0-tclCmdAH.c.Tcl_ErrorObjCmd.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.1.0/tclCmdAH.c.Tcl_ErrorObjCmd.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.1.0/tclCmdAH.c.Tcl_ErrorObjCmd.txt). SHA-256 `21f43cea4236ecd1352f94273d382fde1ad522d66071bec48b53dc4abdcfdcad`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-9.1.0-tclIndexObj.c.Tcl_WrongNumArgs.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.1.0/tclIndexObj.c.Tcl_WrongNumArgs.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.1.0/tclIndexObj.c.Tcl_WrongNumArgs.txt). SHA-256 `656d54729128fdc35823bafa497d48f41c207dbd49175d34ca32a347bd68e182`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-9.1.0-tclVar.c.Tcl_SetObjCmd.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.1.0/tclVar.c.Tcl_SetObjCmd.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/9.1.0/tclVar.c.Tcl_SetObjCmd.txt). SHA-256 `8b5b37638e3ec44c85a380f7f5281fe239429d27b7694fa5f62cd1338ce2a218`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-jim-jim.c.ErrorCommandRegistration.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/jim/jim.c.ErrorCommandRegistration.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/jim/jim.c.ErrorCommandRegistration.txt). SHA-256 `ce5e996424bf79faadefbe6fd8f2882d9987d0a10b0cbfacfdc16e8530ddc94a`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-jim-jim.c.Jim_ErrorCoreCommand.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/jim/jim.c.Jim_ErrorCoreCommand.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/jim/jim.c.Jim_ErrorCoreCommand.txt). SHA-256 `e79e6ae3d5c3a9621ceda39e254ce6877d97cfdbcfff13edc5758a2b00ab01c4`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-jim-jim.c.Jim_SetCoreCommand.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/jim/jim.c.Jim_SetCoreCommand.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/jim/jim.c.Jim_SetCoreCommand.txt). SHA-256 `e8f21bbb126560de0e395aa7440c6f9d208ae5ae8bd39f3ab1bf3e077f284e16`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_error_code_metadata_original-source-windows-jim-jim.c.Jim_WrongNumArgs.txt` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/jim/jim.c.Jim_WrongNumArgs.txt](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/source-windows/jim/jim.c.Jim_WrongNumArgs.txt). SHA-256 `6cd0209f78f0f655372489ee0e58af18248738ff71058abc5c3a129306b4f0fd`. Unchanged original request, fixed ASCII source, launcher, source window, closure, process receipt or complete raw stream. Preparation scope is preserved as requested; measured provider answers are established separately from its prior assumptions.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tcl.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tcl.h). SHA-256 `824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-unix-libtcl8.4.a` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix/libtcl8.4.a](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix/libtcl8.4.a). SHA-256 `532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix/Makefile). SHA-256 `0fb0c580d2402093bb212ad340ea87f88822aa5844a8114e112bec4c990411b1`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_inventory_original-sources-8.4.20-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c). SHA-256 `92695cd7731ce21ab5441fb6ce2d23e5cb44f020256a607397ebe836b5a9b14a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInterp.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclInterp.c). SHA-256 `76c22a84c35e960f7936999f0f19a1aef60c83756a4f3d26d885cfdbd4288230`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclPkg.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclPkg.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclPkg.c). SHA-256 `b2cfb209be52a7caff1fec7cbcdea3970559646555c5a9081e89daecacd57ce9`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_return_provider_executable_audit387-8.4.20-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.4.20/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.4.20/provider.elf). SHA-256 `f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclInt.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclInt.h). SHA-256 `f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclIntDecls.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclIntDecls.h). SHA-256 `ef14c8b968a80d9d4b08b70419afbfcef4dc484d03fb5c3f6c729350e0c45a1e`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclCompile.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclCompile.h). SHA-256 `3d7d3b604522a9c5e673076fab8df53316951da1406892d03fe52dd678e2d0ec`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_compiled_local_formal_source272-8.4.20-tclCompile.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.4.20/tclCompile.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.4.20/tclCompile.c). SHA-256 `0bc55b283d6cb62a4298d4dc30e050b587d5d1b4e7f145e4f644e237dd7639f8`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclExecute.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclExecute.c). SHA-256 `970975c51cf8b78a291b92289cfeac8f023bd126c5d227ae5fa3c7d426a60c77`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_map_prefix312-map-source-8.4.20-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/8.4.20/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/8.4.20/tclNamesp.c). SHA-256 `cd23865ee8d4b0ce81874d519b24de5b84a16c116ed3593ead366946b0512cbb`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclBasic.c). SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclUtil.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclUtil.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclUtil.c). SHA-256 `26a0bb5644999077e8a6a82a7ab1bbcf73787f889a35d7ede5369788c66d2e4c`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclCompCmds.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclCompCmds.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclCompCmds.c). SHA-256 `38626005d02a7445a951f22b55dac182a3ccba76c01834f210fbca94c2dc6214`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_child_colon_publication-full-source-8.4.20-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_child_colon_publication/full-source/8.4.20/tclProc.c](../../../../rust/tcl-registry/tests/data/native_child_colon_publication/full-source/8.4.20/tclProc.c). SHA-256 `bfeecc7c08dabce16946efad9c0e7eb3cacf43d4a5376bf622fa4b69483eb879`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_map_prefix312-map-source-8.4.20-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/8.4.20/tclObj.c](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/8.4.20/tclObj.c). SHA-256 `6e20f047f13e19527f746934b7a9472405d1871e0936687004506f966bb8bd9c`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclStringObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclStringObj.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclStringObj.c). SHA-256 `a86bf9f649ce6de16363984f1d8af398c464984e7d5677543f279ed87e35e62c`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/8.4.20/receipt.json). SHA-256 `0f7234e7d052215ad3586d3f98dbfe640c84868c10d915cda35c612b11dfcf63`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_encoding_utf8_offset307-8.4.20-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/tclCmdAH.c). SHA-256 `4119b635cc3677728baa21df8ee8c683c3f37aa87bf9ca80fcab74eec948fd75`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclVar.c` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclVar.c](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclVar.c). SHA-256 `177628c7637491dffb4ba78f76e5dfdec21331c738c382f47ddb312721f05c9d`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIndexObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIndexObj.c](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIndexObj.c). SHA-256 `babe18e9f596d54d49f16c17b0576723d0d6c580ffb4356e1fca9b26ad6576da`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tcl.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tcl.h). SHA-256 `c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.5.19-unix-libtcl8.5.a` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix/libtcl8.5.a](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix/libtcl8.5.a). SHA-256 `94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.5.19-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix/Makefile). SHA-256 `26ae775d2e4657ecfcb45421cc77c4c26170cb2a21985fbb002eb4791bc766a5`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_inventory_original-sources-8.5.19-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c). SHA-256 `51be49bd7e373aa319cb2affb341853473cbbf089694650bfff7689d3c925d8e`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_event_original-source-8.5.19-tclInterp.c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclInterp.c). SHA-256 `958e2492c5afe9e57a7776cd774ed995224d9c93402b02708ef8c6611588cc04`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclPkg.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclPkg.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclPkg.c). SHA-256 `2365436253e7777a7773f3fa00d9099929fae8406590cd2daebb2914a0a4f0b3`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_return_provider_executable_audit387-8.5.19-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.5.19/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.5.19/provider.elf). SHA-256 `e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_holder_routing205-source-inspection208-8.5.19-tclInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/8.5.19/tclInt.h](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/8.5.19/tclInt.h). SHA-256 `72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclIntDecls.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclIntDecls.h). SHA-256 `e4efb75e3a479b8d816cba3eb4633a449276dc2ac2f1a86d2125cac32dfc0243`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_expression_compiler_name_effects-sources-tcl8.5-tclCompile.h` (source-anchor): [rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.5/tclCompile.h](../../../../rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.5/tclCompile.h). SHA-256 `9ef1b2690de80b9c193d866d8ef8eb3271e57802c91a96f0f0a01f87c1d1a645`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclCompile.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclCompile.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclCompile.c). SHA-256 `8c2ec76dbbe697201bcad79f9e32c0c1db23faf7ca01eb19e08f3ea9e68dd988`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_write_error_lifetime226-failed-request225-original-source-tclExecute.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclExecute.c). SHA-256 `de5706aa9022663636927515066d4630cbfe8ded754dceaca7ae61ee8612b47d`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_holder_routing205-source-inspection208-8.5.19-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/8.5.19/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/8.5.19/tclNamesp.c). SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclBasic.c). SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclUtil.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclUtil.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclUtil.c). SHA-256 `67be2e4aed576288b31069a98a1ddacf8354c87ca1a8c8e85e53421364a989cd`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclCompCmds.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclCompCmds.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclCompCmds.c). SHA-256 `117274255882a35828318ccbfab476c180622310187c5e3a4796867b48ba7414`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_compiled_local_formal_source272-8.5.19-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.5.19/tclProc.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.5.19/tclProc.c). SHA-256 `9adc0419de1af0c81c7718e1e682fb50f4e2873f7b2d07a0f3cbe380f7d2dce6`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_write_error_lifetime226-failed-request225-original-source-tclObj.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclObj.c). SHA-256 `25136b8ad5a833dfecd5e45a2f8f54e52d78e10ff516a5abc61fbdfb7e6ee014`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclStringObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclStringObj.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclStringObj.c). SHA-256 `d014bc85fc8f0d12abbd60116f403f9131dd4e933580833d2b02610e211ab449`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/8.5.19/receipt.json). SHA-256 `5c357fa1bd8f44e02b0b08fca76640fecbe85297493b116cb8e8e0a78aa617e7`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_encoding_utf8_offset307-8.5.19-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/tclCmdAH.c). SHA-256 `cdd328ee0a0d0c3ec7b67da50c90ae22b40591cc52e9492f19d7b257355fcf43`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclVar.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclVar.c). SHA-256 `24f29b0694b5e755f99ab98a0486a63807de9d368b81df65e71ebacb95817768`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIndexObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIndexObj.c](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIndexObj.c). SHA-256 `5ef48cf4d90044d6513c8b452ca7e9070cc3a14e42e83186f8be2d42e90129aa`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-sdk-8.6.18-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/generic/tcl.h). SHA-256 `aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-sdk-8.6.18-unix-libtcl8.6.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/libtcl8.6.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/libtcl8.6.a). SHA-256 `980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-sdk-8.6.18-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/Makefile). SHA-256 `8afb8697cb70b90518876861086bdb43f6e31b5e96e6d8091ae7b1de33d90d7e`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_inventory_original-sources-8.6.18-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c). SHA-256 `26d3d423720df253aefd7ad412e939165282a1f80fd6cd8c42900875d9627a23`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_inventory_original-sources-8.6.18-tclOOInfo.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclOOInfo.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclOOInfo.c). SHA-256 `309603344c3aeebe95832928b473a84ebc001b3d85c8489e15675448035de02b`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_event_original-source-8.6.18-tclInterp.c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclInterp.c). SHA-256 `b2597dfae723709bd21c6dfe9dd0cd4eb6d38f90ea082419b59a2aa2c00881ca`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.6.18-generic-tclPkg.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclPkg.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclPkg.c). SHA-256 `83845b75b4da27d7ae67afd5b15135153d68557681cd9659f75d4d4e027135a2`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_return_provider_executable_audit387-8.6.18-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.6.18/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.6.18/provider.elf). SHA-256 `9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_holder_routing205-source-inspection208-8.6.18-tclInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/8.6.18/tclInt.h](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/8.6.18/tclInt.h). SHA-256 `e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.6.18-generic-tclIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclIntDecls.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclIntDecls.h). SHA-256 `dd303cab02a101f96109c96f626eb91fa5576fe21294a9d80816b06a8014ca26`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_expression_compiler_name_effects-sources-tcl8.6-tclCompile.h` (source-anchor): [rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.6/tclCompile.h](../../../../rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.6/tclCompile.h). SHA-256 `be854eab265b25091f3e5a9b8d268d3aeca520d12382f135e3b3ddede49e6ed9`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.6.18-generic-tclCompile.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclCompile.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclCompile.c). SHA-256 `d1a494975cb56b8850804669e17fb7c5aacc1d70de505ea661f59ca365ab45c8`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_coroutine_compiler_source317-8.6.18-tclCompCmdsGR.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.6.18/tclCompCmdsGR.c](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/8.6.18/tclCompCmdsGR.c). SHA-256 `edda78b970816ff347e08c5f2f957c42b23915bc4fc8410d9d2cfdedba162d8d`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclExecute.c). SHA-256 `2a29cf5e54d1b2ac4ddf56a6f38556bbfb9c8e8913800377c84a6f0bb8caa491`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_oo_original_collision249-request-source-anchors-8.6.18-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclNamesp.c). SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclBasic.c). SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.6.18-generic-tclUtil.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclUtil.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclUtil.c). SHA-256 `452b96a108330de9bf79c52a6266634127b468b704512ffb93e0dc66685df372`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.6.18-generic-tclCompCmds.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclCompCmds.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclCompCmds.c). SHA-256 `c42255a9d4ebb83dfb6f7d956ca411e746a434297b703194e2832eefe705a13e`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_compiled_local_formal_source272-8.6.18-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.6.18/tclProc.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.6.18/tclProc.c). SHA-256 `c1ddd801a69b0e39bf923e97838134489ca4937fb7d081eccc0814b6811f71b3`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_map_prefix312-map-source-8.6.18-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/8.6.18/tclObj.c](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/8.6.18/tclObj.c). SHA-256 `7a7ef8ec85c74581129e8f3bef939e3191a432dcb021e0a16f1cf9971049f90d`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.6.18-generic-tclStringObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclStringObj.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclStringObj.c). SHA-256 `d5cae88e9008d6b9a6101f4c00d7e489fc66fb3d51c86b3b0eb13f7fd8e469b9`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-8.6.18-generic-tclDisassemble.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclDisassemble.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclDisassemble.c). SHA-256 `c116cb5cd9e30a9c48944443dd49e96ad33dcc4d9fb35f8dc824ff64745317fe`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/8.6.18/receipt.json). SHA-256 `9a89b544c51fe11d8dc302119e7047e0cdc0dceb183572c743c430a88c155af1`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_encoding_utf8_offset307-8.6.18-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/tclCmdAH.c). SHA-256 `752473fb5c05d166602ed00b2afff8e123dafd34f4c585628e941b8c5691ae88`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclVar.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclVar.c). SHA-256 `2af24d8fa81db05a2805652ac4359d55ec1e2ef8bbcdad86c4000e25dfd61946`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIndexObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIndexObj.c](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIndexObj.c). SHA-256 `550096cdb69b26ddbcf459c07273db48ad8ed4f5cccd68133559483c4af67c17`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-sdk-9.0.4-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/generic/tcl.h). SHA-256 `eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-sdk-9.0.4-unix-libtcl9.0.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/libtcl9.0.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/libtcl9.0.a). SHA-256 `dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-sdk-9.0.4-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/Makefile). SHA-256 `69f1915c208d66c7e38e6871c7f51c8f7be7c2138b45a5361641f9e91854fea5`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_inventory_original-sources-9.0.4-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c). SHA-256 `af564d5d77f8f70cf120dbd49509a8580130cc89352f52424a01920fb0da4fa7`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_inventory_original-sources-9.0.4-tclOOInfo.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclOOInfo.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclOOInfo.c). SHA-256 `a1ed9fa857c89ba0cc5eea5a6f4e3be82783130823d5182c8e4a4abc5073158b`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_event_original-source-9.0.4-tclInterp.c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclInterp.c). SHA-256 `5c944998c2f89689ef9f591983ea7b0e322dd40756fe5baca641e2258969b125`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclPkg.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclPkg.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclPkg.c). SHA-256 `c89585f9d38a913ce03b9a700987cfe18cb39dec2a24293336eb85b67cd67c8c`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_return_provider_executable_audit387-9.0.4-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/9.0.4/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/9.0.4/provider.elf). SHA-256 `f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_holder_routing205-source-inspection208-9.0.4-tclInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/9.0.4/tclInt.h](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/9.0.4/tclInt.h). SHA-256 `f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclIntDecls.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclIntDecls.h). SHA-256 `96897750658753b43b3de554682bec7249c92e858dbe437eab7de0e26aba9a6a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclCompile.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclCompile.h). SHA-256 `22f512199d2e57370496d8a6713921d9ac57538497a3a64966fa1dee7a6a5dc5`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclCompile.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclCompile.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclCompile.c). SHA-256 `51b709e7bf2c1f25bb7b3ab371c7dcbd6cf6d2bce509931397303cb770ee21f3`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_coroutine_compiler_source317-9.0.4-tclCompCmdsGR.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.0.4/tclCompCmdsGR.c](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.0.4/tclCompCmdsGR.c). SHA-256 `d0a2ad66de4a067375d2486e682a6d66e10ee08e2e91603af2fab63fc397d1db`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclExecute.c). SHA-256 `0c42699cdcf610813ce77e2a6fec505f12d82a87b110950c92824fdf30d3c42a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_oo_original_collision249-request-source-anchors-9.0.4-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclNamesp.c). SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclBasic.c). SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclUtil.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclUtil.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclUtil.c). SHA-256 `13df44c0c262d3445b5c5db138d14c6e45e4993b076387f6e4f272f89d62bcf4`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclCompCmds.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclCompCmds.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclCompCmds.c). SHA-256 `9170e5ff4f14aceb78d1627d4e8ab0b101500dc522e545b60dab855144c0a77a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_compiled_local_formal_source272-9.0.4-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.0.4/tclProc.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.0.4/tclProc.c). SHA-256 `ef5ef608fbb22c68d35317e09637f099380457e5d01ff8e1b67f2a8eeb70c110`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_map_prefix312-map-source-9.0.4-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/9.0.4/tclObj.c](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/9.0.4/tclObj.c). SHA-256 `91a390bd24fbe71108108eeccf9955df0389f0f81dff24f303c32344469e8d4a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclStringObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclStringObj.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclStringObj.c). SHA-256 `d44c80d7de637da41ac841c6c55c25c4ee302efc774c2c4cfe5d8ff73193b7b4`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclDisassemble.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclDisassemble.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclDisassemble.c). SHA-256 `5cc6696914432fe44d8d604289313b5a424f234874018e58f5909be277f01bf5`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/9.0.4/receipt.json). SHA-256 `adcdc0557f5279db1a62ff25ff750400f64280c9c6d0151614ed663aa8fe1fc3`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_encoding_utf8_offset307-9.0.4-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/tclCmdAH.c). SHA-256 `b1fd4aec514b23bbca0a43d99e25a613ec570752e50fadf1caf5c499b1a54ec1`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclVar.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclVar.c). SHA-256 `0ed96dbf7949b0cb84dabd6b9be971ceb00dd97458c87264cc03541c618496fc`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIndexObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIndexObj.c](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIndexObj.c). SHA-256 `5ae0b7f7da756611674d4f80dd04a0646e64674fb3f829805ebebd73866d552a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-sdk-9.1.0-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/generic/tcl.h). SHA-256 `30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-sdk-9.1.0-unix-libtcl9.1.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/libtcl9.1.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/libtcl9.1.a). SHA-256 `513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-sdk-9.1.0-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/Makefile). SHA-256 `c1ecfb5a77697f0dc6f1057f63ff7aabd75b444f62b5954480aff01ff0565ab1`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_inventory_original-sources-9.1.0-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c). SHA-256 `d5b92a5b9c8c923142db6b76f816cbe6ca857255861a3c3dc53787fa9b335e5a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_inventory_original-sources-9.1.0-tclOOInfo.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclOOInfo.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclOOInfo.c). SHA-256 `5f46472c0039bfd89a5ed06aba1df376942136c6ddf02d5c6a0b2cf0bce2a5a9`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_event_original-source-9.1.0-tclInterp.c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclInterp.c). SHA-256 `f7880cfc3cd9787502134b22832475df5ebad0432e66c8fae2281ddd98abe59d`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclPkg.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclPkg.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclPkg.c). SHA-256 `b21d3d4578ac6ad532f53782a8c2aa6d08087e9ecc6638d1b6850b1ca2aca684`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_return_provider_executable_audit387-9.1.0-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/9.1.0/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/9.1.0/provider.elf). SHA-256 `2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_holder_routing205-source-inspection208-9.1.0-tclInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/9.1.0/tclInt.h](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/9.1.0/tclInt.h). SHA-256 `fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclIntDecls.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclIntDecls.h). SHA-256 `78d549e7b85ed9e91df2d44e62db9f2a376b55deb98c55edbbbe53396d45b502`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclCompile.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclCompile.h). SHA-256 `70203ece61377d4cf22a1eae4348d6216bf77372fd7f8a49274f05d8c546d44e`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclCompile.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclCompile.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclCompile.c). SHA-256 `941eaa2c57e4d7d5266134c5b713dca5218d29eea951c89a0676c14b8260eb4f`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_coroutine_compiler_source317-9.1.0-tclCompCmdsGR.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.1.0/tclCompCmdsGR.c](../../../../rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317/9.1.0/tclCompCmdsGR.c). SHA-256 `3656eddb1d634083343f182212b4874aa0939940382cf20252a171b155305fd1`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclExecute.c). SHA-256 `9dd4e9a080136f2ce36ee8f9acf4a30e770cc5fc417519468c785d76013151f4`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_oo_original_collision249-request-source-anchors-9.1.0-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclNamesp.c). SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclBasic.c). SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclUtil.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclUtil.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclUtil.c). SHA-256 `509c893e8aef8f1ae70d0d1e279bbefc60baa758a56d1be184421794f6868e64`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclCompCmds.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclCompCmds.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclCompCmds.c). SHA-256 `66b5f1d2dfab1acb5e9e60634b4d3eb7f01d3deecfb4dc1b95ba077925f0e15e`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_compiled_local_formal_source272-9.1.0-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.1.0/tclProc.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.1.0/tclProc.c). SHA-256 `2c5cf8968a3176aa8e5c316063594186f50109cecd2fbe3c3dff2f3035cefd1a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_map_prefix312-map-source-9.1.0-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/9.1.0/tclObj.c](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/9.1.0/tclObj.c). SHA-256 `58c041dbf20bba3c8fef5782969c661d4d60a6ee0bc915f30e55c19cedb31976`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclStringObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclStringObj.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclStringObj.c). SHA-256 `d1ecd65e375cee3b1ab86111cf5b90fb6bf4d7b5e71448cd361e4026d32a4bf8`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclDisassemble.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclDisassemble.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclDisassemble.c). SHA-256 `9471aba16fecccc2847d7604f2e408fbf2f737124ad5a8601de5f8c7ea911d98`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/9.1.0/receipt.json). SHA-256 `496e47fce5ab0c75d2c56654c7b242652d257dfd2f049869934e5190c6671bb8`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_encoding_utf8_offset307-9.1.0-tclCmdAH.c` (source-anchor): [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/tclCmdAH.c](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/tclCmdAH.c). SHA-256 `56db5fdd8c4bd688cb0beda3b91ad60111adb802fd63c28821055140b582fe04`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclVar.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclVar.c). SHA-256 `270bf098d3a2817b117531713a49367729d030b9b3b05c10dfd4acbba7554759`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIndexObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIndexObj.c](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIndexObj.c). SHA-256 `fd9dfe1a261db55472f88ff9c1a7be53f9c0aee1ea597a6f96ace48b666d8487`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_core_alias_prefix273-jim.h` (source-anchor): [rust/tcl-registry/tests/data/native_jim_core_alias_prefix273/jim.h](../../../../rust/tcl-registry/tests/data/native_jim_core_alias_prefix273/jim.h). SHA-256 `d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-jim-libjim.a` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/libjim.a](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/libjim.a). SHA-256 `a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-jim-Makefile` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/Makefile](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/Makefile). SHA-256 `983469f71a073b757f3ce8869ebf846e7fbb358abd0e5d5d73b07796e366ce77`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_alias_error_extent279-request-jim.c` (source-anchor): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/jim.c](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/jim.c). SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_inventory_original-sources-jim-jim-subcmd.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim-subcmd.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim-subcmd.c). SHA-256 `80f274c72d403c5d7906e22da58cc8f79fd7541e42f34e90e34cc7c5c5248ecb`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_child_alias_inventory298-jim-interp.c` (source-anchor): [rust/tcl-registry/tests/data/native_child_alias_inventory298/jim-interp.c](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/jim-interp.c). SHA-256 `6913a5abf6518b9bdf6b953130ae006e4369207baab2d70bd58b4a55bd0edee9`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-jim-oo.tcl` (input): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/oo.tcl](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/oo.tcl). SHA-256 `42f6b0150881a8d286c1ee9a9b30bdf1b06cf39850c576d9dfc104ef3c0eeef5`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-jim-_oo.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/_oo.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/_oo.c). SHA-256 `a3f26330db47cdfd64377059db2a11b5fcc53d3ae112dfb390686b4f7836eca3`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_child_bootstrap_context303-source-_load-static-exts.c` (source-anchor): [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source/_load-static-exts.c](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source/_load-static-exts.c). SHA-256 `69742674664cfa4ca43495d61075ee681b1c55ed9215eb27ebd2902b69d66979`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_return_provider_executable_audit387-jim-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/jim/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/jim/provider.elf). SHA-256 `e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_alias_introspection274-jim-namespace.c` (source-anchor): [rust/tcl-registry/tests/data/native_jim_alias_introspection274/jim-namespace.c](../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/jim-namespace.c). SHA-256 `22bb6fb5fcea8ce8ec7e3eecbfe5ebd5ee290b9b725101ff4e4a1c0e9abed7f3`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_alias_introspection274-nshelper.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_jim_alias_introspection274/nshelper.tcl](../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/nshelper.tcl). SHA-256 `97c67e98b1bef86c9564044dbed0d5110b38ba0fd416980663101f65b0066627`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-jim-_nshelper.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/_nshelper.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/_nshelper.c). SHA-256 `34b0622f8b6b30947e7409c9666d955fc4d3eb1c5ed8d67ae4699c919a8c4698`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_info_commands_literal_original-original-inputs-jim-jimautoconf.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/jimautoconf.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/jimautoconf.h). SHA-256 `aa112b883e9d9ad1ffc775431524cec57f23d34f643a56ce657951cfae8e3092`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_map_prefix312-map-source-jim-stdlib.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim/stdlib.tcl](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim/stdlib.tcl). SHA-256 `3597b76b785f697242a5354816792ad5fa538dd0a41b3973bdcbcb5edbb8893a`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_ensemble_map_prefix312-map-source-jim-_stdlib.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim/_stdlib.c](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim/_stdlib.c). SHA-256 `593b57426539e77ab00efb961a77c1f8a3845ce0c82f29943516f30db4142751`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/jim/receipt.json). SHA-256 `006a4501536c0ae6e1919ef33f8cfcad38ebd8698b965f90abfcd6d364e32b54`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-probe.tcl` (input): [rust/tcl-registry/tests/data/native_jim_class_source269/probe.tcl](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/probe.tcl). SHA-256 `3206121d2eab608a21401a66bee0f13c6a27da8f827f0565aa34e3ca2357a506`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_source269/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/8.4.20/stdout). SHA-256 `2d800f8db2636ac90f8c131780a0750a1b8db55400127d550edfa8d59770e921`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_c_alias_holder197-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_source269/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/8.5.19/stdout). SHA-256 `a62a70a0fe50fc28f241010018fa1a86375538a2e153b2c03be34b1ebd99bd0d`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_source269/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/8.6.18/stdout). SHA-256 `cd5cde2afdbcee1983518ab5f33f99024875565e2cdc7157bf734777373883a8`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_source269/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/9.0.4/stdout). SHA-256 `578866da922e2d2265a5febf43e251cb99c1da213b96f31f450c1ac573d4b334`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_source269/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/9.1.0/stdout). SHA-256 `9418ff2db46381bd318f51163746b438d2c97619282e6bb324cef9d20710dbea`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_jim_class_source269-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_class_source269/jim/stdout](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/jim/stdout). SHA-256 `5357651f35efde942dcbf4cff5d56617becb239954ab5f20403388c7a3c64185`. Exact original required input or independently observed version provenance; byte-identical published evidence is reused without a new execution.
- `native_error_code_metadata_original-observations.json` (reconfirmation): [rust/tcl-registry/tests/data/native_error_code_metadata_original/observations.json](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/observations.json). SHA-256 `1b75b82013611268f9afb9918dcd5fbd42ae2ebdb58e24e6f41b6abcbdf48ef6`. Derived public-field review of eighteen exact original case rows; raw output is the observation authority.
- `native_error_code_metadata_original-recorded-path-map.json` (provider): [rust/tcl-registry/tests/data/native_error_code_metadata_original/recorded-path-map.json](../../../../rust/tcl-registry/tests/data/native_error_code_metadata_original/recorded-path-map.json). SHA-256 `656a9eff77a835654965701ccb6da8d0e6fbfc2c79a6cbe04fe783722cfcf175`. Checked original path map and required pin identities, including independently observed reused version channels.

## Source inspection

tcl8.4 8.4.20, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdAH.c`, function `Tcl_ErrorObjCmd`, lines 536–652. Full-source SHA-256 `4119b635cc3677728baa21df8ee8c683c3f37aa87bf9ca80fcab74eec948fd75`; snippet SHA-256 `f9cde389a031a9c7ac2e8fc579448b08cbcd7061bcf60f2597d60c8e9c43d947`; retained evidence `native_encoding_utf8_offset307-8.4.20-tclCmdAH.c`.

```text
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_ErrorObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;
    char *info;
    int infoLen;

    if ((objc < 2) || (objc > 4)) {
	Tcl_WrongNumArgs(interp, 1, objv, "message ?errorInfo? ?errorCode?");
	return TCL_ERROR;
    }
    
    if (objc >= 3) {		/* process the optional info argument */
	info = Tcl_GetStringFromObj(objv[2], &infoLen);
	if (infoLen > 0) {
	    Tcl_AddObjErrorInfo(interp, info, infoLen);
	    iPtr->flags |= ERR_ALREADY_LOGGED;
	}
    }
    
    if (objc == 4) {
	Tcl_SetVar2Ex(interp, "errorCode", NULL, objv[3], TCL_GLOBAL_ONLY);
	iPtr->flags |= ERROR_CODE_SET;
    }
    
    Tcl_SetObjResult(interp, objv[1]);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalObjCmd --
 *
 *	This object-based procedure is invoked to process the "eval" Tcl 
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_EvalObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int result;
    register Tcl_Obj *objPtr;
#ifdef TCL_TIP280
    Interp* iPtr = (Interp*) interp;
#endif

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }
    
    if (objc == 2) {
#ifndef TCL_TIP280
	result = Tcl_EvalObjEx(interp, objv[1], TCL_EVAL_DIRECT);
#else
	/* TIP #280. Make argument location available to eval'd script */
	CmdFrame* invoker = iPtr->cmdFramePtr;
	int word          = 1;
	TclArgumentGet (interp, objv[1], &invoker, &word);
	result = TclEvalObjEx(interp, objv[1], TCL_EVAL_DIRECT,
			      invoker, word);
#endif
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result.  Tcl_EvalObjEx will delete
	 * the object when it decrements its refcount after eval'ing it.
	 */
    	objPtr = Tcl_ConcatObj(objc-1, objv+1);
#ifndef TCL_TIP280
	result = Tcl_EvalObjEx(interp, objPtr, TCL_EVAL_DIRECT);
#else
	/* TIP #280. Make invoking context available to eval'd script */
	result = TclEvalObjEx(interp, objPtr, TCL_EVAL_DIRECT, NULL, 0);
#endif
    }
    if (result == TCL_ERROR) {
	char msg[32 + TCL_INTEGER_SPACE];

	sprintf(msg, "\n    (\"eval\" body line %d)", interp->errorLine);
	Tcl_AddObjErrorInfo(interp, msg, -1);
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *

```

tcl8.4 8.4.20, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclVar.c`, function `Tcl_SetObjCmd`, lines 1258–1374. Full-source SHA-256 `177628c7637491dffb4ba78f76e5dfdec21331c738c382f47ddb312721f05c9d`; snippet SHA-256 `95c053949276a7d26a3bf7745298321b570cbc28a5529e4e196473e12cc7acb8`; retained evidence `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclVar.c`.

```text
 *
 * Results:
 *	A standard Tcl result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_SetObjCmd(dummy, interp, objc, objv)
    ClientData dummy;			/* Not used. */
    register Tcl_Interp *interp;	/* Current interpreter. */
    int objc;				/* Number of arguments. */
    Tcl_Obj *CONST objv[];		/* Argument objects. */
{
    Tcl_Obj *varValueObj;

    if (objc == 2) {
	varValueObj = Tcl_ObjGetVar2(interp, objv[1], NULL, TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else if (objc == 3) {

	varValueObj = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
		TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?newValue?");
	return TCL_ERROR;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar --
 *
 *	Change the value of a variable.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not
 *	modify this string. If the write operation was disallowed then NULL
 *	is returned; if the TCL_LEAVE_ERR_MSG flag is set, then an
 *	explanatory message will be left in the interp's result. Note that the
 *	returned string may not be the same as newValue; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	If varName is defined as a local or global variable in interp,
 *	its value is changed to newValue. If varName isn't currently
 *	defined, then a new global variable by that name is created.
 *
 *----------------------------------------------------------------------
 */

#undef Tcl_SetVar
CONST char *
Tcl_SetVar(interp, varName, newValue, flags)
    Tcl_Interp *interp;		/* Command interpreter in which varName is
				 * to be looked up. */
    CONST char *varName;	/* Name of a variable in interp. */
    CONST char *newValue;	/* New value for varName. */
    int flags;			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_APPEND_VALUE,
				 * TCL_LIST_ELEMENT, TCL_LEAVE_ERR_MSG. */
{
    return Tcl_SetVar2(interp, varName, (char *) NULL, newValue, flags);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2 --
 *
 *      Given a two-part variable name, which may refer either to a
 *      scalar variable or an element of an array, change the value
 *      of the variable.  If the named scalar or array or element
 *      doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not
 *	modify this string. If the write operation was disallowed because an
 *	array was expected but not found (or vice versa), then NULL is
 *	returned; if the TCL_LEAVE_ERR_MSG flag is set, then an explanatory
 *	message will be left in the interp's result. Note that the returned
 *	string may not be the same as newValue; this is because variable
 *	traces may modify the variable's value.
 *
 * Side effects:
 *      The value of the given variable is set. If either the array
 *      or the entry didn't exist then a new one is created.
 *
 *----------------------------------------------------------------------
 */

CONST char *
Tcl_SetVar2(interp, part1, part2, newValue, flags)
    Tcl_Interp *interp;         /* Command interpreter in which variable is
                                 * to be looked up. */
    CONST char *part1;          /* If part2 is NULL, this is name of scalar
                                 * variable. Otherwise it is the name of
                                 * an array. */
    CONST char *part2;		/* Name of an element within an array, or
				 * NULL. */

```

tcl8.4 8.4.20, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 430–488. Full-source SHA-256 `babe18e9f596d54d49f16c17b0576723d0d6c580ffb4356e1fca9b26ad6576da`; snippet SHA-256 `8125161d96c388433cfd68cc8b7c730e7a2b3132c8ba6d1ebe2e3320aeb5f053`; retained evidence `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIndexObj.c`.

```text
 *	An error message is generated in interp's result object to
 *	indicate that a command was invoked with the wrong number of
 *	arguments.  The message has the form
 *		wrong # args: should be "foo bar additional stuff"
 *	where "foo" and "bar" are the initial objects in objv (objc
 *	determines how many of these are printed) and "additional stuff"
 *	is the contents of the message argument.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_WrongNumArgs(interp, objc, objv, message)
    Tcl_Interp *interp;			/* Current interpreter. */
    int objc;				/* Number of arguments to print
					 * from objv. */
    Tcl_Obj *CONST objv[];		/* Initial argument objects, which
					 * should be included in the error
					 * message. */
    CONST char *message;		/* Error message to print after the
					 * leading objects in objv. The
					 * message may be NULL. */
{
    Tcl_Obj *objPtr;
    int i;
    register IndexRep *indexRep;

    TclNewObj(objPtr);
    Tcl_SetObjResult(interp, objPtr);
    Tcl_AppendToObj(objPtr, "wrong # args: should be \"", -1);
    for (i = 0; i < objc; i++) {
	/*
	 * If the object is an index type use the index table which allows
	 * for the correct error message even if the subcommand was
	 * abbreviated.  Otherwise, just use the string rep.
	 */
	
	if (objv[i]->typePtr == &tclIndexType) {
	    indexRep = (IndexRep *) objv[i]->internalRep.otherValuePtr;
	    Tcl_AppendStringsToObj(objPtr, EXPAND_OF(indexRep), (char *) NULL);
	} else {
	    Tcl_AppendStringsToObj(objPtr, Tcl_GetString(objv[i]),
		    (char *) NULL);
	}

	/*
	 * Append a space character (" ") if there is more text to follow
	 * (either another element from objv, or the message string).
	 */
	if ((i < (objc - 1)) || message) {
	    Tcl_AppendStringsToObj(objPtr, " ", (char *) NULL);
	}
    }

    if (message) {
	Tcl_AppendStringsToObj(objPtr, message, (char *) NULL);
    }
    Tcl_AppendStringsToObj(objPtr, "\"", (char *) NULL);
}

```

tcl8.5 8.5.19, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdAH.c`, function `Tcl_ErrorObjCmd`, lines 581–697. Full-source SHA-256 `cdd328ee0a0d0c3ec7b67da50c90ae22b40591cc52e9492f19d7b257355fcf43`; snippet SHA-256 `9b02909051ff9f8faba2d3adc092620f73e028fcd8873fa2180d3878b154d908`; retained evidence `native_encoding_utf8_offset307-8.5.19-tclCmdAH.c`.

```text
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_ErrorObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *options, *optName;

    if ((objc < 2) || (objc > 4)) {
	Tcl_WrongNumArgs(interp, 1, objv, "message ?errorInfo? ?errorCode?");
	return TCL_ERROR;
    }

    TclNewLiteralStringObj(options, "-code error -level 0");

    if (objc >= 3) {		/* Process the optional info argument */
	TclNewLiteralStringObj(optName, "-errorinfo");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[2]);
    }

    if (objc >= 4) {		/* Process the optional code argument */
	TclNewLiteralStringObj(optName, "-errorcode");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[3]);
    }

    Tcl_SetObjResult(interp, objv[1]);
    return Tcl_SetReturnOptions(interp, options);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalObjCmd --
 *
 *	This object-based procedure is invoked to process the "eval" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_EvalObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int result;
    register Tcl_Obj *objPtr;
    Interp *iPtr = (Interp *) interp;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {
	/*
	 * TIP #280. Make argument location available to eval'd script.
	 */

	CmdFrame* invoker = iPtr->cmdFramePtr;
	int word          = 1;
	TclArgumentGet (interp, objv[1], &invoker, &word);

	result = TclEvalObjEx(interp, objv[1], TCL_EVAL_DIRECT,
		invoker, word);
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result. Tcl_EvalObjEx will delete the
	 * object when it decrements its refcount after eval'ing it.
	 */

	objPtr = Tcl_ConcatObj(objc-1, objv+1);

	/*
	 * TIP #280. Make invoking context available to eval'd script.
	 */

	result = TclEvalObjEx(interp, objPtr, TCL_EVAL_DIRECT, NULL, 0);
    }
    if (result == TCL_ERROR) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (\"eval\" body line %d)", interp->errorLine));
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_ExitObjCmd --
 *
 *	This procedure is invoked to process the "exit" Tcl command. See the
 *	user documentation for details on what it does.

```

tcl8.5 8.5.19, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclVar.c`, function `Tcl_SetObjCmd`, lines 1500–1616. Full-source SHA-256 `24f29b0694b5e755f99ab98a0486a63807de9d368b81df65e71ebacb95817768`; snippet SHA-256 `375fd43683b9ebbe7921c2e2814068cdec1e9698cc008648fb8de7fdd31e3a49`; retained evidence `native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c`.

```text
 *
 * Results:
 *	A standard Tcl result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

	/* ARGSUSED */
int
Tcl_SetObjCmd(
    ClientData dummy,		/* Not used. */
    register Tcl_Interp *interp,/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *varValueObj;

    if (objc == 2) {
	varValueObj = Tcl_ObjGetVar2(interp, objv[1], NULL,TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else if (objc == 3) {
	varValueObj = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
		TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?newValue?");
	return TCL_ERROR;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar --
 *
 *	Change the value of a variable.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed then NULL is
 *	returned; if the TCL_LEAVE_ERR_MSG flag is set, then an explanatory
 *	message will be left in the interp's result. Note that the returned
 *	string may not be the same as newValue; this is because variable
 *	traces may modify the variable's value.
 *
 * Side effects:
 *	If varName is defined as a local or global variable in interp, its
 *	value is changed to newValue. If varName isn't currently defined, then
 *	a new global variable by that name is created.
 *
 *----------------------------------------------------------------------
 */

#undef Tcl_SetVar
const char *
Tcl_SetVar(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *varName,	/* Name of a variable in interp. */
    const char *newValue,	/* New value for varName. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    return Tcl_SetVar2(interp, varName, NULL, newValue, flags);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable.
 *	If the named scalar or array or element doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed because an array
 *	was expected but not found (or vice versa), then NULL is returned; if
 *	the TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interp's result. Note that the returned string may not be
 *	the same as newValue; this is because variable traces may modify the
 *	variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new one is created.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_SetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    const char *part1,		/* If part2 is NULL, this is name of scalar
				 * variable. Otherwise it is the name of an
				 * array. */
    const char *part2,		/* Name of an element within an array, or
				 * NULL. */
    const char *newValue,	/* New value for variable. */
    int flags)			/* Various flags that tell how to set value:

```

tcl8.5 8.5.19, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 441–557. Full-source SHA-256 `5ef48cf4d90044d6513c8b452ca7e9070cc3a14e42e83186f8be2d42e90129aa`; snippet SHA-256 `1c0f94ce78433aee6e11452dc3a5eba65134a7d9602efcacaa0e49f748376b95`; retained evidence `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIndexObj.c`.

```text
 *	in the interpreter to generate complex multi-part messages by calling
 *	this function repeatedly. This allows the code that knows how to
 *	handle ensemble-related error messages to be kept here while still
 *	generating suitable error messages for commands like [read] and
 *	[socket]. Ideally, this would be done through an extra flags argument,
 *	but that wouldn't be source-compatible with the existing API and it's
 *	a fairly rare requirement anyway.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_WrongNumArgs(
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments to print from objv. */
    Tcl_Obj *const objv[],	/* Initial argument objects, which should be
				 * included in the error message. */
    const char *message)	/* Error message to print after the leading
				 * objects in objv. The message may be
				 * NULL. */
{
    Tcl_Obj *objPtr;
    int i, len, elemLen, flags;
    Interp *iPtr = (Interp *) interp;
    const char *elementStr;

    /*
     * [incr Tcl] does something fairly horrific when generating error
     * messages for its ensembles; it passes the whole set of ensemble
     * arguments as a list in the first argument. This means that this code
     * causes a problem in iTcl if it attempts to correctly quote all
     * arguments, which would be the correct thing to do. We work around this
     * nasty behaviour for now, and hope that we can remove it all in the
     * future...
     */

#ifndef AVOID_HACKS_FOR_ITCL
    int isFirst = 1;		/* Special flag used to inhibit the treating
				 * of the first word as a list element so the
				 * hacky way Itcl generates error messages for
				 * its ensembles will still work. [Bug
				 * 1066837] */
#   define MAY_QUOTE_WORD	(!isFirst)
#   define AFTER_FIRST_WORD	(isFirst = 0)
#else /* !AVOID_HACKS_FOR_ITCL */
#   define MAY_QUOTE_WORD	1
#   define AFTER_FIRST_WORD	(void) 0
#endif /* AVOID_HACKS_FOR_ITCL */

    TclNewObj(objPtr);
    if (iPtr->flags & INTERP_ALTERNATE_WRONG_ARGS) {
	Tcl_AppendObjToObj(objPtr, Tcl_GetObjResult(interp));
	Tcl_AppendToObj(objPtr, " or \"", -1);
    } else {
	Tcl_AppendToObj(objPtr, "wrong # args: should be \"", -1);
    }

    /*
     * Check to see if we are processing an ensemble implementation, and if so
     * rewrite the results in terms of how the ensemble was invoked.
     */

    if (iPtr->ensembleRewrite.sourceObjs != NULL) {
	int toSkip = iPtr->ensembleRewrite.numInsertedObjs;
	int toPrint = iPtr->ensembleRewrite.numRemovedObjs;
	Tcl_Obj *const *origObjv = iPtr->ensembleRewrite.sourceObjs;

	/*
	 * We only know how to do rewriting if all the replaced objects are
	 * actually arguments (in objv) to this function. Otherwise it just
	 * gets too complicated and we'd be better off just giving a slightly
	 * confusing error message...
	 */

	if (objc < toSkip) {
	    goto addNormalArgumentsToMessage;
	}

	/*
	 * Strip out the actual arguments that the ensemble inserted.
	 */

	objv += toSkip;
	objc -= toSkip;

	/*
	 * We assume no object is of index type.
	 */

	for (i=0 ; i<toPrint ; i++) {
	    /*
	     * Add the element, quoting it if necessary.
	     */

	    if (origObjv[i]->typePtr == &indexType) {
		register IndexRep *indexRep =
			origObjv[i]->internalRep.twoPtrValue.ptr1;

		elementStr = EXPAND_OF(indexRep);
		elemLen = strlen(elementStr);
	    } else if (origObjv[i]->typePtr == &tclEnsembleCmdType) {
		register EnsembleCmdRep *ecrPtr =
			origObjv[i]->internalRep.twoPtrValue.ptr1;

		elementStr = ecrPtr->fullSubcmdName;
		elemLen = strlen(elementStr);
	    } else {
		elementStr = TclGetStringFromObj(origObjv[i], &elemLen);
	    }
	    flags = 0;
	    len = TclScanElement(elementStr, elemLen, &flags);

	    if (MAY_QUOTE_WORD && len != elemLen) {
		char *quotedElementStr = TclStackAlloc(interp,
			(unsigned)len + 1);

		len = TclConvertElement(elementStr, elemLen,

```

tcl8.6 8.6.18, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdAH.c`, function `Tcl_ErrorObjCmd`, lines 874–990. Full-source SHA-256 `752473fb5c05d166602ed00b2afff8e123dafd34f4c585628e941b8c5691ae88`; snippet SHA-256 `a19206fb4486172c4aa646ce80515e42b1963c81f9890793e065ba40b21dcffa`; retained evidence `native_encoding_utf8_offset307-8.6.18-tclCmdAH.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_ErrorObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *options, *optName;

    if ((objc < 2) || (objc > 4)) {
	Tcl_WrongNumArgs(interp, 1, objv, "message ?errorInfo? ?errorCode?");
	return TCL_ERROR;
    }

    TclNewLiteralStringObj(options, "-code error -level 0");

    if (objc >= 3) {		/* Process the optional info argument */
	TclNewLiteralStringObj(optName, "-errorinfo");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[2]);
    }

    if (objc >= 4) {		/* Process the optional code argument */
	TclNewLiteralStringObj(optName, "-errorcode");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[3]);
    }

    Tcl_SetObjResult(interp, objv[1]);
    return Tcl_SetReturnOptions(interp, options);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalObjCmd --
 *
 *	This object-based procedure is invoked to process the "eval" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

static int
EvalCmdErrMsg(
    ClientData data[],
    Tcl_Interp *interp,
    int result)
{
    if (result == TCL_ERROR) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (\"eval\" body line %d)", Tcl_GetErrorLine(interp)));
    }
    return result;
}

int
Tcl_EvalObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, TclNREvalObjCmd, dummy, objc, objv);
}

int
TclNREvalObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *objPtr;
    Interp *iPtr = (Interp *) interp;
    CmdFrame *invoker = NULL;
    int word = 0;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {
	/*
	 * TIP #280. Make argument location available to eval'd script.
	 */

	invoker = iPtr->cmdFramePtr;
	word = 1;
	objPtr = objv[1];
	TclArgumentGet(interp, objPtr, &invoker, &word);
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result. Tcl_EvalObjEx will delete the
	 * object when it decrements its refcount after eval'ing it.
	 *
	 * TIP #280. Make invoking context available to eval'd script, done
	 * with the default values.

```

tcl8.6 8.6.18, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclVar.c`, function `Tcl_SetObjCmd`, lines 1497–1613. Full-source SHA-256 `2af24d8fa81db05a2805652ac4359d55ec1e2ef8bbcdad86c4000e25dfd61946`; snippet SHA-256 `6426a93f26b10292570b9da50721fe64949f5498290b705ed76293ac9f92e105`; retained evidence `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_SetObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *varValueObj;

    if (objc == 2) {
	varValueObj = Tcl_ObjGetVar2(interp, objv[1], NULL,TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else if (objc == 3) {
	varValueObj = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
		TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?newValue?");
	return TCL_ERROR;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar --
 *
 *	Change the value of a variable.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed then NULL is
 *	returned; if the TCL_LEAVE_ERR_MSG flag is set, then an explanatory
 *	message will be left in the interp's result. Note that the returned
 *	string may not be the same as newValue; this is because variable
 *	traces may modify the variable's value.
 *
 * Side effects:
 *	If varName is defined as a local or global variable in interp, its
 *	value is changed to newValue. If varName isn't currently defined, then
 *	a new global variable by that name is created.
 *
 *----------------------------------------------------------------------
 */

#undef Tcl_SetVar
const char *
Tcl_SetVar(
    Tcl_Interp *interp,		/* Command interpreter in which varName is to
				 * be looked up. */
    const char *varName,	/* Name of a variable in interp. */
    const char *newValue,	/* New value for varName. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT,
				 * TCL_LEAVE_ERR_MSG. */
{
    Tcl_Obj *varValuePtr, *varNamePtr = Tcl_NewStringObj(varName, -1);

    Tcl_IncrRefCount(varNamePtr);
    varValuePtr = Tcl_ObjSetVar2(interp, varNamePtr, NULL,
	    Tcl_NewStringObj(newValue, -1), flags);
    Tcl_DecrRefCount(varNamePtr);

    if (varValuePtr == NULL) {
	return NULL;
    }
    return TclGetString(varValuePtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable.
 *	If the named scalar or array or element doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed because an array
 *	was expected but not found (or vice versa), then NULL is returned; if
 *	the TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interp's result. Note that the returned string may not be
 *	the same as newValue; this is because variable traces may modify the
 *	variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new one is created.
 *
 *----------------------------------------------------------------------
 */

const char *

```

tcl8.6 8.6.18, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 826–942. Full-source SHA-256 `550096cdb69b26ddbcf459c07273db48ad8ed4f5cccd68133559483c4af67c17`; snippet SHA-256 `6521cc4830fab0f2cc0cf206b252858fb1298a31c4a9acd4e5ade7d045d77203`; retained evidence `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIndexObj.c`.

```text
 *	in the interpreter to generate complex multi-part messages by calling
 *	this function repeatedly. This allows the code that knows how to
 *	handle ensemble-related error messages to be kept here while still
 *	generating suitable error messages for commands like [read] and
 *	[socket]. Ideally, this would be done through an extra flags argument,
 *	but that wouldn't be source-compatible with the existing API and it's
 *	a fairly rare requirement anyway.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_WrongNumArgs(
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments to print from objv. */
    Tcl_Obj *const objv[],	/* Initial argument objects, which should be
				 * included in the error message. */
    const char *message)	/* Error message to print after the leading
				 * objects in objv. The message may be
				 * NULL. */
{
    Tcl_Obj *objPtr;
    int i, len, elemLen;
    char flags;
    Interp *iPtr = (Interp *)interp;
    const char *elementStr;

    /*
     * [incr Tcl] does something fairly horrific when generating error
     * messages for its ensembles; it passes the whole set of ensemble
     * arguments as a list in the first argument. This means that this code
     * causes a problem in iTcl if it attempts to correctly quote all
     * arguments, which would be the correct thing to do. We work around this
     * nasty behaviour for now, and hope that we can remove it all in the
     * future...
     */

#ifndef AVOID_HACKS_FOR_ITCL
    int isFirst = 1;		/* Special flag used to inhibit the treating
				 * of the first word as a list element so the
				 * hacky way Itcl generates error messages for
				 * its ensembles will still work. [Bug
				 * 1066837] */
#   define MAY_QUOTE_WORD	(!isFirst)
#   define AFTER_FIRST_WORD	(isFirst = 0)
#else /* !AVOID_HACKS_FOR_ITCL */
#   define MAY_QUOTE_WORD	1
#   define AFTER_FIRST_WORD	(void) 0
#endif /* AVOID_HACKS_FOR_ITCL */

    TclNewObj(objPtr);
    if (iPtr->flags & INTERP_ALTERNATE_WRONG_ARGS) {
	iPtr->flags &= ~INTERP_ALTERNATE_WRONG_ARGS;
	Tcl_AppendObjToObj(objPtr, Tcl_GetObjResult(interp));
	Tcl_AppendToObj(objPtr, " or \"", -1);
    } else {
	Tcl_AppendToObj(objPtr, "wrong # args: should be \"", -1);
    }

    /*
     * If processing an an ensemble implementation, rewrite the results in
     * terms of how the ensemble was invoked.
     */

    if (iPtr->ensembleRewrite.sourceObjs != NULL) {
	int toSkip = iPtr->ensembleRewrite.numInsertedObjs;
	int toPrint = iPtr->ensembleRewrite.numRemovedObjs;
	Tcl_Obj *const *origObjv = TclEnsembleGetRewriteValues(interp);

	/*
	 * Only do rewrite the command if all the replaced objects are
	 * actually arguments (in objv) to this function. Otherwise it just
	 * gets too complicated and it's to just give a slightly
	 * confusing error message...
	 */

	if (objc < toSkip) {
	    goto addNormalArgumentsToMessage;
	}

	/*
	 * Strip out the actual arguments that the ensemble inserted.
	 */

	objv += toSkip;
	objc -= toSkip;

	/*
	 * Assume no object is of index type.
	 */

	for (i=0 ; i<toPrint ; i++) {
	    /*
	     * Add the element, quoting it if necessary.
	     */

	    if (origObjv[i]->typePtr == &indexType) {
		IndexRep *indexRep =
			(IndexRep *)origObjv[i]->internalRep.twoPtrValue.ptr1;

		elementStr = EXPAND_OF(indexRep);
		elemLen = strlen(elementStr);
	    } else {
		elementStr = TclGetStringFromObj(origObjv[i], &elemLen);
	    }
	    flags = 0;
	    len = TclScanElement(elementStr, elemLen, &flags);

	    if (MAY_QUOTE_WORD && len != elemLen) {
		char *quotedElementStr = (char *)TclStackAlloc(interp, len + 1);

		len = TclConvertElement(elementStr, elemLen,
			quotedElementStr, flags);
		Tcl_AppendToObj(objPtr, quotedElementStr, len);
		TclStackFree(interp, quotedElementStr);
	    } else {
		Tcl_AppendToObj(objPtr, elementStr, elemLen);

```

tcl9.0 9.0.4, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdAH.c`, function `Tcl_ErrorObjCmd`, lines 875–991. Full-source SHA-256 `b1fd4aec514b23bbca0a43d99e25a613ec570752e50fadf1caf5c499b1a54ec1`; snippet SHA-256 `3aa71c1110953d8b9c577f83aa86f6d0da07dec66e70ebaa0b7111a58b9dbeb1`; retained evidence `native_encoding_utf8_offset307-9.0.4-tclCmdAH.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_ErrorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *options, *optName;

    if ((objc < 2) || (objc > 4)) {
	Tcl_WrongNumArgs(interp, 1, objv, "message ?errorInfo? ?errorCode?");
	return TCL_ERROR;
    }

    TclNewLiteralStringObj(options, "-code error -level 0");

    if (objc >= 3) {		/* Process the optional info argument */
	TclNewLiteralStringObj(optName, "-errorinfo");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[2]);
    }

    if (objc >= 4) {		/* Process the optional code argument */
	TclNewLiteralStringObj(optName, "-errorcode");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[3]);
    }

    Tcl_SetObjResult(interp, objv[1]);
    return Tcl_SetReturnOptions(interp, options);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalObjCmd --
 *
 *	This object-based procedure is invoked to process the "eval" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

static int
EvalCmdErrMsg(
    TCL_UNUSED(void **),
    Tcl_Interp *interp,
    int result)
{
    if (result == TCL_ERROR) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (\"eval\" body line %d)", Tcl_GetErrorLine(interp)));
    }
    return result;
}

int
Tcl_EvalObjCmd(
    void *clientData,
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, TclNREvalObjCmd, clientData, objc, objv);
}

int
TclNREvalObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *objPtr;
    Interp *iPtr = (Interp *) interp;
    CmdFrame *invoker = NULL;
    int word = 0;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {
	/*
	 * TIP #280. Make argument location available to eval'd script.
	 */

	invoker = iPtr->cmdFramePtr;
	word = 1;
	objPtr = objv[1];
	TclArgumentGet(interp, objPtr, &invoker, &word);
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result. Tcl_EvalObjEx will delete the
	 * object when it decrements its refcount after eval'ing it.
	 *
	 * TIP #280. Make invoking context available to eval'd script, done
	 * with the default values.

```

tcl9.0 9.0.4, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclVar.c`, function `Tcl_SetObjCmd`, lines 1485–1601. Full-source SHA-256 `0ed96dbf7949b0cb84dabd6b9be971ceb00dd97458c87264cc03541c618496fc`; snippet SHA-256 `237f5aa94832d4839fcbcdbe9436c8a8b9ef15217649019293cbf984b8601fdd`; retained evidence `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_SetObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *varValueObj;

    if (objc == 2) {
	varValueObj = Tcl_ObjGetVar2(interp, objv[1], NULL,TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else if (objc == 3) {
	varValueObj = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
		TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?newValue?");
	return TCL_ERROR;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable.
 *	If the named scalar or array or element doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed because an array
 *	was expected but not found (or vice versa), then NULL is returned; if
 *	the TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interp's result. Note that the returned string may not be
 *	the same as newValue; this is because variable traces may modify the
 *	variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new one is created.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_SetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    const char *part1,		/* If part2 is NULL, this is name of scalar
				 * variable. Otherwise it is the name of an
				 * array. */
    const char *part2,		/* Name of an element within an array, or
				 * NULL. */
    const char *newValue,	/* New value for variable. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT, or
				 * TCL_LEAVE_ERR_MSG. */
{
    Tcl_Obj *varValuePtr = Tcl_SetVar2Ex(interp, part1, part2,
	    Tcl_NewStringObj(newValue, -1), flags);

    if (varValuePtr == NULL) {
	return NULL;
    }
    return TclGetString(varValuePtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2Ex --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable
 *	to a new Tcl object value. If the named scalar or array or element
 *	doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the write operation was disallowed because an array was
 *	expected but not found (or vice versa), then NULL is returned; if the
 *	TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interpreter's result. Note that the returned object may
 *	not be the same one referenced by newValuePtr; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new variable is created.
 *
 *	The reference count is decremented for any old value of the variable
 *	and incremented for its new value. If the new value for the variable
 *	is not the same one referenced by newValuePtr (perhaps as a result of

```

tcl9.0 9.0.4, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 796–912. Full-source SHA-256 `5ae0b7f7da756611674d4f80dd04a0646e64674fb3f829805ebebd73866d552a`; snippet SHA-256 `77260f12eea3e8630d845887d77c26c2cbe74a04c085ad677caed22400698223`; retained evidence `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIndexObj.c`.

```text
 *	in the interpreter to generate complex multi-part messages by calling
 *	this function repeatedly. This allows the code that knows how to
 *	handle ensemble-related error messages to be kept here while still
 *	generating suitable error messages for commands like [read] and
 *	[socket]. Ideally, this would be done through an extra flags argument,
 *	but that wouldn't be source-compatible with the existing API and it's
 *	a fairly rare requirement anyway.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_WrongNumArgs(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,			/* Number of arguments to print from objv. */
    Tcl_Obj *const objv[],	/* Initial argument objects, which should be
				 * included in the error message. */
    const char *message)	/* Error message to print after the leading
				 * objects in objv. The message may be
				 * NULL. */
{
    Tcl_Obj *objPtr;
    Tcl_Size i, len, elemLen;
    char flags;
    Interp *iPtr = (Interp *)interp;
    const char *elementStr;

    TclNewObj(objPtr);
    if (iPtr->flags & INTERP_ALTERNATE_WRONG_ARGS) {
	iPtr->flags &= ~INTERP_ALTERNATE_WRONG_ARGS;
	Tcl_AppendObjToObj(objPtr, Tcl_GetObjResult(interp));
	Tcl_AppendToObj(objPtr, " or \"", TCL_INDEX_NONE);
    } else {
	Tcl_AppendToObj(objPtr, "wrong # args: should be \"", TCL_INDEX_NONE);
    }

    /*
     * If processing an ensemble implementation, rewrite the results in
     * terms of how the ensemble was invoked.
     */

    if (iPtr->ensembleRewrite.sourceObjs != NULL) {
	Tcl_Size toSkip = iPtr->ensembleRewrite.numInsertedObjs;
	Tcl_Size toPrint = iPtr->ensembleRewrite.numRemovedObjs;
	Tcl_Obj *const *origObjv = TclEnsembleGetRewriteValues(interp);

	/*
	 * Only do rewrite the command if all the replaced objects are
	 * actually arguments (in objv) to this function. Otherwise it just
	 * gets too complicated and it's to just give a slightly
	 * confusing error message...
	 */

	if (objc < toSkip) {
	    goto addNormalArgumentsToMessage;
	}

	/*
	 * Strip out the actual arguments that the ensemble inserted.
	 */

	objv += toSkip;
	objc -= toSkip;

	/*
	 * Assume no object is of index type.
	 */

	for (i=0 ; i<toPrint ; i++) {
	    /*
	     * Add the element, quoting it if necessary.
	     */
	    const Tcl_ObjInternalRep *irPtr;

	    if ((irPtr = TclFetchInternalRep(origObjv[i], &tclIndexType))) {
		IndexRep *indexRep = (IndexRep *)irPtr->twoPtrValue.ptr1;

		elementStr = EXPAND_OF(indexRep);
		elemLen = strlen(elementStr);
	    } else {
		elementStr = TclGetStringFromObj(origObjv[i], &elemLen);
	    }
	    flags = 0;
	    len = TclScanElement(elementStr, elemLen, &flags);

	    if (len != elemLen) {
		char *quotedElementStr = (char *)TclStackAlloc(interp, len + 1);

		len = TclConvertElement(elementStr, elemLen,
			quotedElementStr, flags);
		Tcl_AppendToObj(objPtr, quotedElementStr, len);
		TclStackFree(interp, quotedElementStr);
	    } else {
		Tcl_AppendToObj(objPtr, elementStr, elemLen);
	    }

	    /*
	     * Add a space if the word is not the last one (which has a
	     * moderately complex condition here).
	     */

	    if (i + 1 < toPrint || objc!=0 || message!=NULL) {
		Tcl_AppendStringsToObj(objPtr, " ", (char *)NULL);
	    }
	}
    }

    /*
     * Now add the arguments (other than those rewritten) that the caller took
     * from its calling context.
     */

  addNormalArgumentsToMessage:
    for (i = 0; i < objc; i++) {
	/*
	 * If the object is an index type, use the index table which allows for
	 * the correct error message even if the subcommand was abbreviated.

```

tcl9.1 9.1.0, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdAH.c`, function `Tcl_ErrorObjCmd`, lines 896–1012. Full-source SHA-256 `56db5fdd8c4bd688cb0beda3b91ad60111adb802fd63c28821055140b582fe04`; snippet SHA-256 `21f43cea4236ecd1352f94273d382fde1ad522d66071bec48b53dc4abdcfdcad`; retained evidence `native_encoding_utf8_offset307-9.1.0-tclCmdAH.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_ErrorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Obj *options, *optName;

    if ((objc < 2) || (objc > 4)) {
	Tcl_WrongNumArgs(interp, 1, objv, "message ?errorInfo? ?errorCode?");
	return TCL_ERROR;
    }

    TclNewLiteralStringObj(options, "-code error -level 0");

    if (objc >= 3) {		/* Process the optional info argument */
	TclNewLiteralStringObj(optName, "-errorinfo");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[2]);
    }

    if (objc >= 4) {		/* Process the optional code argument */
	TclNewLiteralStringObj(optName, "-errorcode");
	Tcl_ListObjAppendElement(NULL, options, optName);
	Tcl_ListObjAppendElement(NULL, options, objv[3]);
    }

    Tcl_SetObjResult(interp, objv[1]);
    return Tcl_SetReturnOptions(interp, options);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_EvalObjCmd --
 *
 *	This object-based procedure is invoked to process the "eval" Tcl
 *	command. See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl object result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *----------------------------------------------------------------------
 */

static int
EvalCmdErrMsg(
    TCL_UNUSED(void **),
    Tcl_Interp *interp,
    int result)
{
    if (result == TCL_ERROR) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (\"eval\" body line %d)", Tcl_GetErrorLine(interp)));
    }
    return result;
}

int
Tcl_EvalObjCmd(
    void *clientData,
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    return Tcl_NRCallObjProc2(interp, TclNREvalObjCmd, clientData, objc, objv);
}

int
TclNREvalObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Obj *objPtr;
    Interp *iPtr = (Interp *) interp;
    CmdFrame *invoker = NULL;
    Tcl_Size word = 0;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "arg ?arg ...?");
	return TCL_ERROR;
    }

    if (objc == 2) {
	/*
	 * TIP #280. Make argument location available to eval'd script.
	 */

	invoker = iPtr->cmdFramePtr;
	word = 1;
	objPtr = objv[1];
	TclArgumentGet(interp, objPtr, &invoker, &word);
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result. Tcl_EvalObjEx will delete the
	 * object when it decrements its refcount after eval'ing it.
	 *
	 * TIP #280. Make invoking context available to eval'd script, done
	 * with the default values.

```

tcl9.1 9.1.0, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclVar.c`, function `Tcl_SetObjCmd`, lines 1539–1655. Full-source SHA-256 `270bf098d3a2817b117531713a49367729d030b9b3b05c10dfd4acbba7554759`; snippet SHA-256 `8b5b37638e3ec44c85a380f7f5281fe239429d27b7694fa5f62cd1338ce2a218`; retained evidence `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
 *	user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result value.
 *
 * Side effects:
 *	A variable's value may be changed.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_SetObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Obj *varValueObj;

    if (objc == 2) {
	varValueObj = Tcl_ObjGetVar2(interp, objv[1], NULL,TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else if (objc == 3) {
	varValueObj = Tcl_ObjSetVar2(interp, objv[1], NULL, objv[2],
		TCL_LEAVE_ERR_MSG);
	if (varValueObj == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, varValueObj);
	return TCL_OK;
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "varName ?newValue?");
	return TCL_ERROR;
    }
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2 --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable.
 *	If the named scalar or array or element doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the malloc'ed string which is the character
 *	representation of the variable's new value. The caller must not modify
 *	this string. If the write operation was disallowed because an array
 *	was expected but not found (or vice versa), then NULL is returned; if
 *	the TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interp's result. Note that the returned string may not be
 *	the same as newValue; this is because variable traces may modify the
 *	variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new one is created.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_SetVar2(
    Tcl_Interp *interp,		/* Command interpreter in which variable is to
				 * be looked up. */
    const char *part1,		/* If part2 is NULL, this is name of scalar
				 * variable. Otherwise it is the name of an
				 * array. */
    const char *part2,		/* Name of an element within an array, or
				 * NULL. */
    const char *newValue,	/* New value for variable. */
    int flags)			/* Various flags that tell how to set value:
				 * any of TCL_GLOBAL_ONLY, TCL_NAMESPACE_ONLY,
				 * TCL_APPEND_VALUE, TCL_LIST_ELEMENT, or
				 * TCL_LEAVE_ERR_MSG. */
{
    Tcl_Obj *varValuePtr = Tcl_SetVar2Ex(interp, part1, part2,
	    Tcl_NewStringObj(newValue, -1), flags);

    if (varValuePtr == NULL) {
	return NULL;
    }
    return TclGetString(varValuePtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_SetVar2Ex --
 *
 *	Given a two-part variable name, which may refer either to a scalar
 *	variable or an element of an array, change the value of the variable
 *	to a new Tcl object value. If the named scalar or array or element
 *	doesn't exist then create one.
 *
 * Results:
 *	Returns a pointer to the Tcl_Obj holding the new value of the
 *	variable. If the write operation was disallowed because an array was
 *	expected but not found (or vice versa), then NULL is returned; if the
 *	TCL_LEAVE_ERR_MSG flag is set, then an explanatory message will be
 *	left in the interpreter's result. Note that the returned object may
 *	not be the same one referenced by newValuePtr; this is because
 *	variable traces may modify the variable's value.
 *
 * Side effects:
 *	The value of the given variable is set. If either the array or the
 *	entry didn't exist then a new variable is created.
 *
 *	The reference count is decremented for any old value of the variable
 *	and incremented for its new value. If the new value for the variable
 *	is not the same one referenced by newValuePtr (perhaps as a result of

```

tcl9.1 9.1.0, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclIndexObj.c`, function `Tcl_WrongNumArgs`, lines 798–914. Full-source SHA-256 `fd9dfe1a261db55472f88ff9c1a7be53f9c0aee1ea597a6f96ace48b666d8487`; snippet SHA-256 `656d54729128fdc35823bafa497d48f41c207dbd49175d34ca32a347bd68e182`; retained evidence `native_error_code_metadata_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIndexObj.c`.

```text
 *	in the interpreter to generate complex multi-part messages by calling
 *	this function repeatedly. This allows the code that knows how to
 *	handle ensemble-related error messages to be kept here while still
 *	generating suitable error messages for commands like [read] and
 *	[socket]. Ideally, this would be done through an extra flags argument,
 *	but that wouldn't be source-compatible with the existing API and it's
 *	a fairly rare requirement anyway.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_WrongNumArgs(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments to print from objv. */
    Tcl_Obj *const *objv,	/* Initial argument objects, which should be
				 * included in the error message. */
    const char *message)	/* Error message to print after the leading
				 * objects in objv. The message may be
				 * NULL. */
{
    Tcl_Obj *objPtr;
    Tcl_Size i, len, elemLen;
    char flags;
    Interp *iPtr = (Interp *)interp;
    const char *elementStr;

    TclNewObj(objPtr);
    if (iPtr->flags & INTERP_ALTERNATE_WRONG_ARGS) {
	iPtr->flags &= ~INTERP_ALTERNATE_WRONG_ARGS;
	Tcl_AppendObjToObj(objPtr, Tcl_GetObjResult(interp));
	Tcl_AppendToObj(objPtr, " or \"", TCL_INDEX_NONE);
    } else {
	Tcl_AppendToObj(objPtr, "wrong # args: should be \"", TCL_INDEX_NONE);
    }

    /*
     * If processing an ensemble implementation, rewrite the results in
     * terms of how the ensemble was invoked.
     */

    if (iPtr->ensembleRewrite.sourceObjs != NULL) {
	Tcl_Size toSkip = iPtr->ensembleRewrite.numInsertedObjs;
	Tcl_Size toPrint = iPtr->ensembleRewrite.numRemovedObjs;
	Tcl_Obj *const *origObjv = TclEnsembleGetRewriteValues(interp);

	/*
	 * Only do rewrite the command if all the replaced objects are
	 * actually arguments (in objv) to this function. Otherwise it just
	 * gets too complicated and it's to just give a slightly
	 * confusing error message...
	 */

	if (objc < toSkip) {
	    goto addNormalArgumentsToMessage;
	}

	/*
	 * Strip out the actual arguments that the ensemble inserted.
	 */

	objv += toSkip;
	objc -= toSkip;

	/*
	 * Assume no object is of index type.
	 */

	for (i=0 ; i<toPrint ; i++) {
	    /*
	     * Add the element, quoting it if necessary.
	     */
	    const Tcl_ObjInternalRep *irPtr;

	    if ((irPtr = TclFetchInternalRep(origObjv[i], &tclIndexType))) {
		IndexRep *indexRep = (IndexRep *)irPtr->twoPtrValue.ptr1;

		elementStr = EXPAND_OF(indexRep);
		elemLen = strlen(elementStr);
	    } else {
		elementStr = TclGetStringFromObj(origObjv[i], &elemLen);
	    }
	    flags = 0;
	    len = TclScanElement(elementStr, elemLen, &flags);

	    if (len != elemLen) {
		char *quotedElementStr = (char *)TclStackAlloc(interp, len + 1);

		len = TclConvertElement(elementStr, elemLen,
			quotedElementStr, flags);
		Tcl_AppendToObj(objPtr, quotedElementStr, len);
		TclStackFree(interp, quotedElementStr);
	    } else {
		Tcl_AppendToObj(objPtr, elementStr, elemLen);
	    }

	    /*
	     * Add a space if the word is not the last one (which has a
	     * moderately complex condition here).
	     */

	    if (i + 1 < toPrint || objc!=0 || message!=NULL) {
		Tcl_AppendStringsToObj(objPtr, " ", (char *)NULL);
	    }
	}
    }

    /*
     * Now add the arguments (other than those rewritten) that the caller took
     * from its calling context.
     */

  addNormalArgumentsToMessage:
    for (i = 0; i < objc; i++) {
	/*
	 * If the object is an index type, use the index table which allows for
	 * the correct error message even if the subcommand was abbreviated.

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_ErrorCoreCommand`, lines 16582–16698. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `e79e6ae3d5c3a9621ceda39e254ce6877d97cfdbcfff13edc5758a2b00ab01c4`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
    }
    else {
        if (listPtr == (Jim_Obj *)EOF) {
            Jim_SetResult(interp, Jim_NewListObj(interp, 0, 0));
            return JIM_OK;
        }
        Jim_SetResult(interp, listPtr);
    }
    return JIM_OK;
}

/* [error] */
static int Jim_ErrorCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_SetResult(interp, argv[1]);
    if (argc == 3) {
        JimSetStackTrace(interp, argv[2]);
        return JIM_ERR;
    }
    return JIM_ERR;
}

/* [lrange] */
static int Jim_LrangeCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *objPtr;

    if ((objPtr = Jim_ListRange(interp, argv[1], argv[2], argv[3])) == NULL)
        return JIM_ERR;
    Jim_SetResult(interp, objPtr);
    return JIM_OK;
}

/* [lrepeat] */
static int Jim_LrepeatCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *objPtr;
    jim_wide count;

    if (argc < 2 || Jim_GetWideExpr(interp, argv[1], &count) != JIM_OK || count < 0) {
        return JIM_USAGE;
    }
    if (count == 0 || argc == 2) {
        Jim_SetEmptyResult(interp);
        return JIM_OK;
    }

    argc -= 2;
    argv += 2;

    objPtr = Jim_NewListObj(interp, NULL, 0);
    ListEnsureLength(objPtr, argc * count);
    while (count--) {
        ListInsertElements(objPtr, -1, argc, argv);
    }

    Jim_SetResult(interp, objPtr);
    return JIM_OK;
}

char **Jim_GetEnviron(void)
{
#if defined(HAVE__NSGETENVIRON)
    return *_NSGetEnviron();
#elif defined(_environ)
    return _environ;
#else
    #if !defined(NO_ENVIRON_EXTERN)
    extern char **environ;
    #endif
    return environ;
#endif
}

void Jim_SetEnviron(char **env)
{
#if defined(HAVE__NSGETENVIRON)
    *_NSGetEnviron() = env;
#elif defined(_environ)
    _environ = env;
#else
    #if !defined(NO_ENVIRON_EXTERN)
    extern char **environ;
    #endif

    environ = env;
#endif
}

/* [env] */
static int Jim_EnvCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    const char *key;
    const char *val;

    if (argc == 1) {
        char **e = Jim_GetEnviron();

        int i;
        Jim_Obj *listObjPtr = Jim_NewListObj(interp, NULL, 0);

        for (i = 0; e[i]; i++) {
            const char *equals = strchr(e[i], '=');

            if (equals) {
                Jim_ListAppendElement(interp, listObjPtr, Jim_NewStringObj(interp, e[i],
                        equals - e[i]));
                Jim_ListAppendElement(interp, listObjPtr, Jim_NewStringObj(interp, equals + 1, -1));
            }
        }

        Jim_SetResult(interp, listObjPtr);
        return JIM_OK;
    }

    key = Jim_String(argv[1]);
    val = getenv(key);

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SetCoreCommand`, lines 12719–12835. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `e8f21bbb126560de0e395aa7440c6f9d208ae5ae8bd39f3ab1bf3e077f284e16`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
static int Jim_SubCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    return JimSubDivHelper(interp, argc, argv, JIM_EXPROP_SUB);
}

/* [/] */
static int Jim_DivCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    return JimSubDivHelper(interp, argc, argv, JIM_EXPROP_DIV);
}

/* [set] */
static int Jim_SetCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    if (argc == 2) {
        Jim_Obj *objPtr;

        objPtr = Jim_GetVariable(interp, argv[1], JIM_ERRMSG);
        if (!objPtr)
            return JIM_ERR;
        Jim_SetResult(interp, objPtr);
        return JIM_OK;
    }
    /* argc == 3 case. */
    if (Jim_SetVariable(interp, argv[1], argv[2]) != JIM_OK)
        return JIM_ERR;
    Jim_SetResult(interp, argv[2]);
    return JIM_OK;
}

/* [unset]
 *
 * unset ?-nocomplain? ?--? ?varName ...?
 */
static int Jim_UnsetCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int i = 1;
    int complain = 1;

    while (i < argc) {
        if (Jim_CompareStringImmediate(interp, argv[i], "--")) {
            i++;
            break;
        }
        if (Jim_CompareStringImmediate(interp, argv[i], "-nocomplain")) {
            complain = 0;
            i++;
            continue;
        }
        break;
    }

    while (i < argc) {
        if (Jim_UnsetVariable(interp, argv[i], complain ? JIM_ERRMSG : JIM_NONE) != JIM_OK
            && complain) {
            return JIM_ERR;
        }
        i++;
    }

    Jim_SetEmptyResult(interp);
    return JIM_OK;
}

/**
 * All commands that support break, continue from a loop (while, loop, foreach, for)
 * use this to check for break_level.
 *
 * If break_level is > 0, decrements the break_level and returns 1.
 * Otherwise returns 0
 */
static int JimCheckLoopRetcode(Jim_Interp *interp, int retval)
{
    if (retval == JIM_BREAK || retval == JIM_CONTINUE) {
        if (--interp->break_level > 0) {
            return 1;
        }
    }
    return 0;
}

/* [while] */
static int Jim_WhileCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    /* The general purpose implementation of while starts here */
    while (1) {
        int boolean = 0, retval;

        if ((retval = Jim_GetBoolFromExpr(interp, argv[1], &boolean)) != JIM_OK)
            return retval;
        if (!boolean)
            break;

        if ((retval = Jim_EvalObj(interp, argv[2])) != JIM_OK) {
            if (JimCheckLoopRetcode(interp, retval)) {
                return retval;
            }
            switch (retval) {
                case JIM_BREAK:
                    goto out;
                case JIM_CONTINUE:
                    continue;
                default:
                    return retval;
            }
        }
    }
  out:
    Jim_SetEmptyResult(interp);
    return JIM_OK;
}

/* [for] */
static int Jim_ForCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retval;
    int boolean = 1;

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_WrongNumArgs`, lines 12324–12440. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `6cd0209f78f0f655372489ee0e58af18248738ff71058abc5c3a129306b4f0fd`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
    Jim_IncrRefCount(listObjPtr);
    objPtr = Jim_ListJoin(interp, listObjPtr, " ", 1);
    Jim_DecrRefCount(interp, listObjPtr);
    Jim_IncrRefCount(objPtr);

    return objPtr;
}

 /* This function should not be necessary for simple commands.
  * Instead set minargs/maxargs and a usage string when registering the command and
  * optionally return JIM_USAGE from the command proc to generate a usage message.
  */
void Jim_WrongNumArgs(Jim_Interp *interp, int argc, Jim_Obj *const *argv, const char *msg)
{
    Jim_Obj *objPtr = JimJoinCmdArgs(interp, argc, argv);
    if (*msg) {
        Jim_SetResultFormatted(interp, "wrong # args: should be \"%#s %s\"", objPtr, msg);
    }
    else {
        Jim_SetResultFormatted(interp, "wrong # args: should be \"%#s\"", objPtr);
    }
    Jim_DecrRefCount(interp, objPtr);
}

/**
 * Calculates the taint of the given objects (skipping NULL entries).
 */
int Jim_CalcTaint(int argc, Jim_Obj *const *argv)
{
    int taint = 0;
#ifdef JIM_TAINT
    int i;
    for (i = 0; i < argc; i++) {
        if (argv[i]) {
            taint |= argv[i]->taint;
        }
    }
#endif
    return taint;
}

void Jim_SetTaintError(Jim_Interp *interp, int cmdargs, Jim_Obj *const *argv)
{
#ifdef JIM_TAINT
    Jim_Obj *objPtr = JimJoinCmdArgs(interp, cmdargs, argv);
    Jim_SetResultFormatted(interp, "%#s: tainted data", objPtr);
    Jim_DecrRefCount(interp, objPtr);
    Jim_SetGlobalVariableStr(interp, "errorCode", Jim_NewStringObj(interp, "TAINTED", -1));
#endif
}

/**
 * May add the key and/or value to the list.
 */
typedef void JimHashtableIteratorCallbackType(Jim_Interp *interp, Jim_Obj *listObjPtr,
    Jim_Obj *keyObjPtr, void *value, Jim_Obj *patternObjPtr, int type);

#define JimTrivialMatch(pattern)    (strpbrk((pattern), "*[?\\") == NULL)

/**
 * For each key of the hash table 'ht' with object keys that
 * matches the glob pattern (all if NULL), invoke the callback to add entries to a list.
 * Returns the list.
 */
static Jim_Obj *JimHashtablePatternMatch(Jim_Interp *interp, Jim_HashTable *ht, Jim_Obj *patternObjPtr,
    JimHashtableIteratorCallbackType *callback, int type)
{
    Jim_HashEntry *he;
    Jim_Obj *listObjPtr = Jim_NewListObj(interp, NULL, 0);

    /* Check for the non-pattern case. We can do this much more efficiently. */
    if (patternObjPtr && JimTrivialMatch(Jim_String(patternObjPtr))) {
        he = Jim_FindHashEntry(ht, patternObjPtr);
        if (he) {
            callback(interp, listObjPtr, Jim_GetHashEntryKey(he), Jim_GetHashEntryVal(he),
                patternObjPtr, type);
        }
    }
    else {
        Jim_HashTableIterator htiter;
        JimInitHashTableIterator(ht, &htiter);
        while ((he = Jim_NextHashEntry(&htiter)) != NULL) {
            callback(interp, listObjPtr, Jim_GetHashEntryKey(he), Jim_GetHashEntryVal(he),
                patternObjPtr, type);
        }
    }
    return listObjPtr;
}

/* Keep these in order */
#define JIM_CMDLIST_COMMANDS 1
#define JIM_CMDLIST_PROCS 2
#define JIM_CMDLIST_ALIASES 4
#define JIM_CMDLIST_CHANNELS 8

#define JIM_CMDLIST_ALL 0x1000

/**
 * Adds matching command names (procs, channels) to the list.
 */
static void JimCommandMatch(Jim_Interp *interp, Jim_Obj *listObjPtr,
    Jim_Obj *keyObj, void *value, Jim_Obj *patternObj, int type)
{
    Jim_Cmd *cmdPtr = (Jim_Cmd *)value;
    int match = 1;

    if ((type & JIM_CMDLIST_PROCS) && !(cmdPtr->flags & JIM_CMD_ISPROC)) {
        /* not a proc */
        return;
    }
    if ((type & JIM_CMDLIST_CHANNELS) && !(cmdPtr->flags & JIM_CMD_ISCHANNEL)) {
        /* not a channel */
        return;
    }
    if ((type & JIM_CMDLIST_ALIASES) && !(cmdPtr->flags & JIM_CMD_ISALIAS)) {
        /* not an alias */
        return;

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel is measured independently.`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `error command argc 1..2 registration`, lines 16837–16856. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `ce5e996424bf79faadefbe6fd8f2882d9987d0a10b0cbfacfdc16e8530ddc94a`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
    {"apply", Jim_ApplyCoreCommand, 1, -1, "lambdaExpr ?arg ...?" },
    {"break", Jim_BreakCoreCommand, 0, 1, "?level?" },
    {"catch", Jim_CatchCoreCommand, 1, -1, "?-?no?code ... --? script ?resultVarName? ?optionVarName?" },
    {"concat", Jim_ConcatCoreCommand, 0, -1, "?arg ...?" },
    {"continue", Jim_ContinueCoreCommand, 0, 1, "?level?" },
#if defined(JIM_DEBUG_COMMAND) && !defined(JIM_BOOTSTRAP)
    {"debug", Jim_DebugCoreCommand, 1, -1, "subcommand ?arg ...?" },
#endif /* JIM_DEBUG_COMMAND && !JIM_BOOTSTRAP */
    {"dict", Jim_DictCoreCommand, 1, -1, "subcommand ?arg ...?"},
    {"env", Jim_EnvCoreCommand, 0, 2, "?varName? ?default?" },
    {"error", Jim_ErrorCoreCommand, 1, 2, "message ?stacktrace?" },
    {"eval", Jim_EvalCoreCommand, 1, -1, "arg ?arg ...?" },
    {"exists", Jim_ExistsCoreCommand, 1, 2, "?-command|-proc|-alias|-channel|-var? name" },
    {"exit", Jim_ExitCoreCommand, 0, 1, "?exitCode?" },
#ifdef JIM_COMPAT
    {"expr", Jim_ExprCoreCommand, 1, -1, "expression ?...?" },
#else
    {"expr", Jim_ExprCoreCommand, 1, 1, "expression" },
#endif
    {"for", Jim_ForCoreCommand, 4, 4, "start test next body" },

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-vm/tests/embed_api_e2e.rs](../../../../rust/tcl-vm/tests/embed_api_e2e.rs), `an_error_a_host_took_as_its_own_leaves_error_code_and_error_info`: Keep caller-owned caught completion metadata and arbitrary message text separate in the software embedding consumer; actual Native430 release-specific public producers remain independent.
- [rust/tcl-vm/tests/embed_api_e2e.rs](../../../../rust/tcl-vm/tests/embed_api_e2e.rs), `an_error_a_host_took_as_its_own_leaves_error_code_and_error_info` (linked): Software caller-owned completion metadata and arbitrary message text, independently from Native430 original script cases.

These source bindings establish no executed assertion result; exact software outcomes belong to the independently pinned Rust validation receipts.

- [rust/tcl-registry/src/native_wrong_arguments.rs](../../../../rust/tcl-registry/src/native_wrong_arguments.rs), `InvocationDialect::native_wrong_arguments_protocol`: Select exact measured error-code policy by independent actual release: NONE for C8.4/C8.5/Jim and TCL WRONGARGS for C8.6/C9; software origin/String assertions do not attest public original headers.
- [rust/tcl-registry/src/native_wrong_arguments.rs](../../../../rust/tcl-registry/src/native_wrong_arguments.rs), `native_wrong_arguments::tests::measured_native_codes_have_independent_origin` (linked): Selected software error-code protocol compared with exact six-provider public original catch {set} metadata. Existing origin/String producer assertions remain software controls; original public scripts do not observe physical headers. This links the software assertion definition only. Independent actual execution receipts retain their own failures and scope; no whole-provider or specialised compilation pass is inferred.

These source bindings establish no executed assertion result; exact software outcomes belong to the independently pinned Rust validation receipts.

## Replay

The unchanged request/probe/launcher, six complete original process receipts and stdout/stderr, exact required original CLI/source/header/configuration/static archive pins, reused version receipts and their complete source/streams, fresh version queries and nineteen source windows are retained with checked hashes and a publication path map. Replay uses the original argv and explicit environment overlay in each receipt; recorded absolute paths and unrecorded inherited environment are not reconstructed. A new launch is a new observation, not an assertion, compile recipe, object/header identity or generic diagnostic permission. Publication itself launches no compiler, provider or Rust test.
