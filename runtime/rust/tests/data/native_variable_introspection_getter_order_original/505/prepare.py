from pathlib import Path
import hashlib
import json
import shutil

base = Path(__file__).resolve().parent
repo = Path('/workspace/tcl-lsp')
prior = json.loads(Path('/workspace/.proofs/native-mathop-constructor424/request.json').read_bytes())
sha = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
compiler = Path(shutil.which('cc')).resolve()
required = {str(compiler): sha(compiler)}
# Selected system link inputs and compiler tools are pinned before launching.
# The original successful link map and ldd receipt independently close actual use.
system_roots = [Path('/usr/lib/x86_64-linux-gnu'), Path('/lib/x86_64-linux-gnu')]
for root in system_roots:
    for pattern in ['libc.so*', 'libc_nonshared.a', 'libm.so*', 'libm-*.so', 'libmvec.so*',
                    'libdl.a', 'libdl.so*', 'libpthread.a', 'libpthread.so*', 'libz.so*',
                    'libssl.so*', 'libcrypto.so*', 'libgcc_s.so*', 'ld-linux*.so*',
                    'crt1.o', 'Scrt1.o', 'crti.o', 'crtn.o']:
        for path in root.glob(pattern):
            if path.is_file():
                resolved = path.resolve(); required[str(resolved)] = sha(resolved)
for root in Path('/usr/lib/gcc/x86_64-linux-gnu').glob('*'):
    if root.name != '14':
        continue
    for pattern in ['crt*.o', 'libgcc*.a', 'libgcc_s.so', 'cc1', 'collect2', 'include/**/*.h']:
        for path in root.glob(pattern):
            if path.is_file():
                resolved = path.resolve(); required[str(resolved)] = sha(resolved)
# System include roots selected by this compiler are retained before launch;
# each actual compiler -MD dependency must later join an exact pin.
for path in list(Path('/usr/include').glob('*.h')) + list(Path('/usr/include/x86_64-linux-gnu').rglob('*.h')):
    if path.is_file():
        selected = path.resolve(); required[str(selected)] = sha(selected)
for path in Path('/usr/lib/gcc/x86_64-linux-gnu/14/include-fixed').rglob('*.h'):
    if path.is_file():
        selected = path.resolve(); required[str(selected)] = sha(selected)
for tool in ['ld', 'as', 'ldd']:
    path = Path(shutil.which(tool)).resolve(); required[str(path)] = sha(path)

# All six predecessor dependency streams are retained source evidence only.
# New provider execution is independent and never splices predecessor C results.
predecessor = Path('/workspace/.proofs/native-naming-argument-order499d')
for stream in sorted((predecessor / 'providers').glob('*/dependencies.stdout')):
    required[str(stream)] = sha(stream)
    import re
    for line in stream.read_text().splitlines():
        for selected in re.findall(r'(?<!\S)(/[^\s()]+)', line):
            path = Path(selected).resolve()
            required[str(path)] = sha(path)
required[str(predecessor / 'request.json')] = sha(predecessor / 'request.json')
for name in ['probe.c', 'launch.py', 'prepare.py', 'cases.json']:
    required[str(base / name)] = sha(base / name)
builds = []
windows = []
for previous in prior['provider_builds']:
    name = previous['provider']
    command = [str(compiler) if item == 'cc' else
               str(base / 'probe.c') if item.endswith('/native-mathop-constructor424/probe.c') else
               str(base / 'providers' / name / 'probe.elf') if item.endswith('/probe.elf') else item
               for item in previous['command']]
    command[1:1] = ['-MD', '-MF', str(base / 'providers' / name / 'compile.d')]
    command.insert(-2, '-Wl,-Map,' + str(base / 'providers' / name / 'link.map'))
    pins = {path: sha(path) for path in previous['required_sha256']
            if not path.endswith('/receipt.json')}
    if name == 'jim':
        source_root = Path('/workspace/.proofs/native-providers/jimtcl')
        for pattern in ['*.h', 'nshelper.tcl', 'jim-namespace.c', 'jim.c', '_nshelper.c']:
            for path in source_root.glob(pattern):
                pins[str(path)] = sha(path)
        sources = [('jim.c', 'static int Jim_InfoCoreCommand(', 220),
                   ('jim.c', 'static int Jim_IncrCoreCommand(', 55),
                   ('jim.c', 'static int Jim_UnsetCoreCommand(', 42),
                   ('jim.c', 'int Jim_EvalObjVector(', 80),
                   ('jim.c', 'Jim_Obj *Jim_GetGlobalVariableStr(', 30),
                   ('jim-array.c', 'static int array_cmd_set(', 65),
                   ('jim.h', 'JIM_EXPORT int Jim_EvalObjVector', 5)]
    else:
        source_root = repo / 'tmp' / ('tcl' + name)
        for path in (source_root / 'generic').glob('*.h'):
            pins[str(path)] = sha(path)
        for pattern in ['Makefile', 'tclConfig.sh']:
            path = source_root / 'unix' / pattern
            if path.exists():
                pins[str(path)] = sha(path)
        for path in (source_root / 'library').rglob('*'):
            if path.is_file():
                pins[str(path)] = sha(path)
        sources = [('generic/tclCmdIL.c', 'InfoDefaultCmd(', 112),
                   ('generic/tclCmdIL.c', 'Tcl_IncrObjCmd(', 120),
                   ('generic/tclVar.c', 'Tcl_UnsetObjCmd(', 85),
                   ('generic/tclVar.c', 'TclIncrObjVar2(', 100),
                   ('generic/tclVar.c', 'TclPtrIncrObjVar(', 160),
                   ('generic/tclVar.c', 'ArraySetCmd(', 220),
                   ('generic/tclVar.c', 'Tcl_ArrayObjCmd(', 680),
                   ('generic/tclVar.c', 'ArrayForNRCmd(', 100),
                   ('generic/tclVar.c', 'ArrayForLoopCallback(', 150),
                   ('generic/tclVar.c', 'Tcl_ConstObjCmd(', 90),
                   ('generic/tclVar.c', 'Tcl_ObjSetVar2(', 105),
                   ('generic/tclBasic.c', 'Tcl_EvalObjv(', 110)]
    for relative, needle, count in sources:
        path = source_root / relative
        data = path.read_bytes()
        lines = data.splitlines(keepends=True)
        candidates = [i for i, line in enumerate(lines) if needle.encode() in line]
        # Definitions have the symbol at the beginning of the source line.
        chosen = next((i for i in candidates if lines[i].startswith(needle.encode())),
                      candidates[-1] if candidates else None)
        if chosen is None:
            windows.append({'provider': name, 'source': str(path), 'source_sha256': sha(path),
                            'symbol': needle, 'status': 'not-found-in-selected-source'})
            continue
        first = max(0, chosen - 1)
        last = min(len(lines), chosen + count)
        output = base / 'source-windows' / name / (Path(relative).name + '-' + str(chosen + 1) + '.txt')
        output.parent.mkdir(parents=True, exist_ok=True)
        original = b''.join(lines[first:last])
        output.write_bytes(original)
        pins[str(path)] = sha(path)
        required[str(output)] = sha(output)
        windows.append({'provider': name, 'source': str(path), 'source_sha256': sha(path),
                        'first_line': first + 1, 'last_line': last,
                        'source_byte_start': sum(map(len, lines[:first])),
                        'source_byte_end': sum(map(len, lines[:last])),
                        'window': str(output), 'window_sha256': sha(output), 'symbol': needle})
    required.update(pins)
    builds.append({'provider': name, 'command': command,
                   'environment': previous['environment'], 'required_sha256': pins,
                   'bootstrap': 'Fresh original interpreter per case; actual distribution Init '
                                'or Jim core plus static extensions; actual public version emitted per case.'})

request = {
    'question_id': 'naming.invocation.original-variable-and-introspection-getter-order',
    'document': 'docs/design/analysis/name-resolution-proofs/invocation-original-variable-and-introspection-getter-order.md',
    'fixture_root': 'rust/tcl-registry/tests/data/native_variable_introspection_getter_order_original',
    'problem_statement': 'Which original argument String getters are reached, in what order, and which prefix variable effects are committed by info default, array set/for, const, incr, unset and info exists? Earlier Guest rejection must remain separate from later getter or mutation.',
    'status': 'prepared-not-compiled-or-executed',
    'predecessor_scope': 'Only exact successful499d dependency streams are reused to pin the selected actual runtime library closure. This new request compiles and executes all six independently; no predecessor observations are spliced.',
    'compiler': str(compiler),
    'probe_file': str(base / 'probe.c'), 'probe_sha256': sha(base / 'probe.c'),
    'launcher_file': str(base / 'launch.py'), 'launcher_sha256': sha(base / 'launch.py'),
    'providers': ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0', 'jim'],
    'bigip': 'not-tested', 'provider_builds': builds, 'required_sha256': required,
    'source_windows': windows,
    'cases': json.loads((base / 'cases.json').read_bytes()),
    'route': 'The source evaluator constructs the actual argument vector for a genuine extension shim. The shim immediately delegates those same borrowed headers to Tcl_EvalObjv/Jim_EvalObjVector. No operand byte serialization, catch, eval rewrite or worker script compilation is attributed. Script setup and poststate use separate ordinary public evaluation channels.',
    'observers': {
        'BIRTH': 'Genuine extension object original pointer, absent String, reference count and exact binary-decoded payload.',
        'EVENT': 'Reached genuine updateString callback, its original pointer/String-before/refs/payload and selected public global variable target/arr(k)/arr(m)/survivor presence and String residency. Jim array callback axes emit explicit NA because dictionary-sugar lookup could materialise a different original key; no absence claim is inferred. Observer calls only public non-error-reporting variable getters with resident names, never value String/eval/result publication. Its reads are actual observer effects, not a claim of an unobserved worker table state.',
        'ROW': 'Actual evaluation code/result hex and cumulative getter count, separated version/setup/worker/poststate phases. Public result capture can itself reach getters and remains inside the reported cumulative ROW count.',
        'unsupported': 'Every unavailable info default/array for/const worker remains an actual code/result row, including unreached lazy operands. No distribution-wide command availability or C/Jim equivalence is inferred.'},
    'source_bytes': 'Physical ASCII/LF probe; lazy payload bytes decoded from explicit hex. Persistent global original headers and pure Lists preserve operand ownership across setup and worker. Setup ends in resident READY; lookup context is the actual global frame and is separate from retained header ownership.',
    'purpose_limits': ['No Native token/generation/header/cache/bytecode or compiled source equivalence from public rows.', 'No Runtime Host/refusal/Scope semantic proof from these original callbacks.', 'Refcounts/pointers are actual process observations only, with extension/public invocation owners retained explicitly.', 'Array/constant unsupported purposes and all Guest/setup/query failures remain explicit.', 'Poststate getters and their effects are separate from worker and internal callback observations.', 'BIG-IP remains not-tested; no C recipe donated to Jim.'],
    'required_receipts': 'Fresh output directory; compiler version plus six actual compile/dependency/execute channels, full stdout/stderr/exit/timeout and actual version/Init; exact successful own compile executable identity before/after execution, link map, actual -MD header closure and actual dependency closure; all request/source/header/config/archive/library/launcher pins unchanged before/after every process; 18 closed fresh cases and final marker per executed provider.'}
(base / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
manifest = {'status': 'prepared-only', 'question_id': request['question_id'],
            'request_sha256': sha(base / 'request.json'),
            'files': [{'path': str(path), 'sha256': sha(path)} for path in sorted(base.rglob('*'))
                      if path.is_file() and path.name != 'manifest.json']}
(base / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print('Prepared original inputs:', len(required), 'source windows:', len(windows))
