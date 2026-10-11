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
for tool in ['ld', 'as', 'ldd']:
    path = Path(shutil.which(tool)).resolve(); required[str(path)] = sha(path)

# All six predecessor dependency streams are retained source evidence only.
# New provider execution is independent and never splices predecessor C results.
predecessor = Path('/workspace/.proofs/native-naming-argument-order499c')
for stream in sorted((predecessor / 'providers').glob('*/dependencies.stdout')):
    required[str(stream)] = sha(stream)
    import re
    for line in stream.read_text().splitlines():
        for selected in re.findall(r'(?<!\S)(/[^\s()]+)', line):
            path = Path(selected).resolve()
            required[str(path)] = sha(path)
required[str(predecessor / 'request.json')] = sha(predecessor / 'request.json')
for name in ['probe.c', 'launch.py', 'prepare.py']:
    required[str(base / name)] = sha(base / name)
builds = []
windows = []
for previous in prior['provider_builds']:
    name = previous['provider']
    command = [str(compiler) if item == 'cc' else
               str(base / 'probe.c') if item.endswith('/native-mathop-constructor424/probe.c') else
               str(base / 'providers' / name / 'probe.elf') if item.endswith('/probe.elf') else item
               for item in previous['command']]
    command.insert(-2, '-Wl,-Map,' + str(base / 'providers' / name / 'link.map'))
    pins = {path: sha(path) for path in previous['required_sha256']
            if not path.endswith('/receipt.json')}
    if name == 'jim':
        source_root = Path('/workspace/.proofs/native-providers/jimtcl')
        for pattern in ['*.h', 'nshelper.tcl', 'jim-namespace.c', 'jim.c', '_nshelper.c']:
            for path in source_root.glob(pattern):
                pins[str(path)] = sha(path)
        sources = [('nshelper.tcl', 'proc {namespace import}', 42),
                   ('jim-namespace.c', 'JimNamespaceCmd', 60),
                   ('jim.c', 'Jim_Obj *Jim_NewObj', 40),
                   ('jim.h', 'typedef struct Jim_ObjType', 12)]
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
        sources = [('generic/tclNamesp.c', 'NamespaceExportCmd(', 62),
                   ('generic/tclNamesp.c', 'NamespaceImportCmd(', 110),
                   ('generic/tclNamesp.c', 'NamespaceForgetCmd(', 50),
                   ('generic/tclProc.c', 'Tcl_ProcObjCmd(', 94)]
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
    'question_id': 'naming.invocation.original-naming-argument-getter-and-effect-order',
    'document': 'docs/design/analysis/name-resolution-proofs/invocation-original-naming-argument-getter-and-effect-order.md',
    'fixture_root': 'rust/tcl-registry/tests/data/native_naming_argument_getter_effect_order_original',
    'problem_statement': 'Which original naming argument String getters are reached, and what '
                         'current export/import state exists at each reached getter, when export '
                         'clear/append, import/forget and procedure name/formal validation commit or fail? '
                         'Pre-reading all operands can reach a later getter before an earlier Guest '
                         'failure or mutation and can hide the actual prefix commitment.',
    'status': 'prepared-not-compiled-or-executed',
    'predecessor_scope': '499c independently closed all six processes; Jim workers 1-14 stopped before lazy getters because setup-local variables were absent in a new namespace eval frame. This request uses retained global original headers and recompiles/re-executes all six independently; no predecessor observations are spliced.',
    'compiler': str(compiler),
    'probe_file': str(base / 'probe.c'), 'probe_sha256': sha(base / 'probe.c'),
    'launcher_file': str(base / 'launch.py'), 'launcher_sha256': sha(base / 'launch.py'),
    'providers': ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0', 'jim'],
    'bigip': 'not-tested', 'provider_builds': builds, 'required_sha256': required,
    'source_windows': windows,
    'cases': {
        '0': 'namespace export query, no original lazy operands',
        '1': 'leading clear then qualified invalid first pattern; later getter reachability',
        '2': 'valid first export then invalid second and unreached third',
        '3': 'invalid first export without clear, unreached second',
        '4': 'later -clear is a pattern, not leading control',
        '5': 'valid first import then missing namespace and later valid source',
        '6': 'leading force plus same import sequence',
        '7': 'unqualified first import pattern plus later valid source',
        '8': 'forget valid first import then missing namespace and later valid import',
        '9': 'unknown procedure-name namespace before invalid formal list/lazy body',
        '10': 'valid procedure name, invalid formal list and lazy body; reached order measured independently',
        '11': 'valid name/empty formal list/lazy body; invocation observed separately',
        '12': 'custom payload binary NUL: ok then NUL then ::bad export suffix',
        '13': 'custom payload binary NUL: valid import name then NUL then ::bad suffix',
        '14': 'custom payload UTF8 bytes c3a9 in first export then invalid second and later third'},
    'observers': {
        'BIRTH': 'Original extension object constructor NULL String/header refs and exact payload bytes; no argv getter forced.',
        'EVENT': 'Reached genuine updateString callback, original String presence/refcount/payload, '
                 'selected public direct export-list snapshot and public direct ::d::a/b/p command existence.',
        'ROW': 'Actual public evaluation code/result hex and cumulative reached getter count, '
               'separate version/setup/worker/exports/commands/invoke channels.',
        'C84_export_list': 'Selected public tcl.h/tclDecls.h lack Tcl_AppendExportList. Callback observer '
                           'emits explicit NA; original public follow-up namespace export still measured. '
                           'No private prototype, manufactured namespace absence or before-getter export-list claim.',
        'Jim_export_list': 'No C export-list axis; explicit NA at callback. Original scripted namespace '
                           'export/forget/import results remain independently measured, including unsupported/ignored purposes.'},
    'source_bytes': 'Physical probe.c is ASCII/LF. Lazy payloads are decoded from explicit hex; '
                    'cases12/13 binary-produced NUL and case14 binary-produced c3a9 UTF8 bytes '
                    'are not source NUL/nonASCII characters. Setup ends in resident READY '
                    'so setup result capture does not force a lazy operand. No eval/catch rewriting of workers.',
    'purpose_limits': ['No generic command/token/header/cache equivalence from public rows.',
                       'No host refusal or Runtime Scope semantics inferred from original Tcl callbacks.',
                       'No body choice/duplication/bytecode/opcode equivalence from lazy body String reachability.',
                       'Public direct C export-list observer missing in8.4 remains NA despite source interpretation.',
                       'Command existence is a public name query, not an original binding-generation identity.',
                       'All Guest setup/worker/query failures and Jim distribution/helper distinctions retained.'],
    'required_receipts': 'Fresh provider output directories only; compiler version, six actual compile, dependency inspection and execute whole streams/exits/timeouts; '
                         'actual probe ELF SHA/stat BEFORE and AFTER execute with successful own compile producer association, actual link-map/archive and dynamic dependency closure, actual per-case versions/Init; '
                         'all input/source/launcher hashes verified before/after every process; '
                         '15 closed-case records and final closure marker per executed provider.'}
(base / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
manifest = {'status': 'prepared-only', 'question_id': request['question_id'],
            'request_sha256': sha(base / 'request.json'),
            'files': [{'path': str(path), 'sha256': sha(path)} for path in sorted(base.rglob('*'))
                      if path.is_file() and path.name != 'manifest.json']}
(base / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print('Prepared original inputs:', len(required), 'source windows:', len(windows))
