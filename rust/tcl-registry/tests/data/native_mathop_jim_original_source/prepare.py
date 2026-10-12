from pathlib import Path
import hashlib
import json

P = Path(__file__).resolve().parent
REPO = Path('/workspace/tcl-lsp')
JIM = Path('/workspace/.proofs/native-providers/jimtcl')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


previous = json.loads(Path('/workspace/.proofs/native-mathop-constructor424/request.json').read_text())
provider = next(row for row in previous['provider_builds'] if row['provider'] == 'jim')
required = dict(provider['required_sha256'])
executable = JIM / 'jimsh'
source_windows = []
for file, anchor, first, last in [
    ('jimsh.c', 'Jim_InitStaticExtensions actual CLI entry', 72, 125),
    ('_load-static-exts.c', 'Jim_namespaceInit and Jim_nshelperInit', 1, 70),
    ('_nshelper.c', 'generated Jim_nshelperInit complete source', 1, 150),
    ('nshelper.tcl', 'namespace info scripted helper', 87, 142),
    ('jim.c', 'INFO_COMMANDS genuine namespace-aware helper forwarding', 16070, 16150),
    ('Makefile', 'selected static namespace/nshelper extension roster', 1, 80),
]:
    source = JIM / file
    lines = source.read_bytes().splitlines(keepends=True)
    last = min(last, len(lines))
    window = P / 'source-windows' / (file + '.txt')
    window.parent.mkdir(parents=True, exist_ok=True)
    window.write_bytes(b''.join(lines[first - 1:last]))
    required[str(source)] = sha(source)
    required[str(window)] = sha(window)
    source_windows.append({'source': str(source), 'source_sha256': sha(source), 'anchor': anchor,
                           'first_line': first, 'last_line': last,
                           'window_file': str(window), 'window_sha256': sha(window)})

operations = []
for family in ('identity', 'compilation'):
    root = REPO / 'rust/tcl-cmd-core/tests/data' / ('native_mathop_' + family)
    rows = root / 'rows.txt'
    old_manifest = root / 'manifest.json'
    required[str(rows)] = sha(rows)
    required[str(old_manifest)] = sha(old_manifest)
    captured = json.loads(old_manifest.read_text())['captures']
    for line in rows.read_bytes().splitlines():
        fields = line.split(b'\t')
        if fields[0] != b'jim':
            continue
        index = int(fields[1])
        source = bytes.fromhex(fields[4].decode('ascii'))
        old = next(row for row in captured if row['engine'] == 'jim' and row['case'] == index)
        if old['source'].encode('utf-8') != source:
            raise RuntimeError('Raw-source and original manifest bytes differ')
        output = P / 'inputs' / family / str(index)
        output.mkdir(parents=True, exist_ok=True)
        raw = output / 'source.tcl'
        raw.write_bytes(source)
        wrapper = output / 'wrapper.tcl'
        # Counted hex decoding transports the exact source; eval is used only
        # for this original public script completion observation, never as an
        # assertion about native specialised compilation or physical headers.
        wrapper.write_bytes((
            'set __n435_source [binary format H* {' + source.hex() + '}]\n'
            'set __n435_code [catch {eval $__n435_source} __n435_result]\n'
            'binary scan $__n435_result H* __n435_hex\n'
            'puts "CASE_RESULT $__n435_code $__n435_hex"\n'
            'puts CLOSED_CASE\n'
        ).encode('ascii'))
        required[str(raw)] = sha(raw)
        required[str(wrapper)] = sha(wrapper)
        operations.append({'kind': 'original-case', 'family': family, 'case': index,
                           'command': [str(executable), str(wrapper)],
                           'source_file': str(raw), 'source_sha256': sha(raw),
                           'wrapper_file': str(wrapper), 'wrapper_sha256': sha(wrapper),
                           'source_hex': source.hex(), 'source_length': len(source),
                           'source_contains_non_ascii': any(byte >= 128 for byte in source),
                           'source_contains_nul': 0 in source,
                           'source_line_feed_count': source.count(b'\n'),
                           'old_capture_manifest': str(old_manifest), 'old_capture_index': index,
                           'old_executable': old['binary'], 'old_executable_sha256': old['binary_sha256'],
                           'expected_code': int(fields[2]), 'expected_result_hex': fields[3].decode('ascii')})

metadata = P / 'metadata.tcl'
metadata.write_bytes(b'''foreach {label script} {
    VERSION {info patchlevel}
    DECLARATION_ARGS {info args {namespace info}}
    DECLARATION_BODY {info body {namespace info}}
    GUARD_COMMANDS {info commands ::tcl::mathop::+}
} {
    set code [catch $script result]
    binary scan $result H* hex
    puts "$label $code $hex"
}
puts CLOSED_METADATA
''')
required[str(metadata)] = sha(metadata)
operations.append({'kind': 'independent-helper-metadata', 'command': [str(executable), str(metadata)],
                   'source_file': str(metadata), 'source_sha256': sha(metadata)})
required[str(P / 'launch.py')] = sha(P / 'launch.py')
request = {
    'question_id': 'naming.mathop.original-jim-source-and-helper-context',
    'document': 'docs/design/analysis/name-resolution-proofs/mathop-original-jim-source-and-helper-context.md',
    'fixture_root': 'rust/tcl-registry/tests/data/native_mathop_jim_original_source',
    'problem_statement': 'Which result bytes and catch codes do the unchanged 24 original mathop scripts produce under the pinned current Jim distribution, and what genuine namespace-info helper declaration supplies their absolute command guard?',
    'scope': '24 byte-identical original Jim source inputs in independent fresh CLI processes and one separate public helper/formals/body/version process. Only public catch codes/results are compared with old captured rows. The old different CLI hash remains separate and grants no inferred source/library roster. No specialised native compile, command-table equivalence, argument cache, result-header, backend test or physical lifetime claim.',
    'provider': 'jim', 'executable': str(executable), 'executable_sha256': sha(executable),
    'environment': provider['environment'], 'version_receipt': provider['version_receipt'],
    'version_receipt_sha256': provider['version_receipt_sha256'],
    'required_sha256': required, 'source_windows': source_windows, 'operations': operations,
    'expected_case_count': 24, 'expected_process_count': 25,
    'launcher_file': str(P / 'launch.py'), 'launcher_sha256': sha(P / 'launch.py'),
    'status': 'PREPARED; Root executes original provider processes independently from software controls',
}
if sum(operation['kind'] == 'original-case' for operation in operations) != 24:
    raise RuntimeError('Expected exactly 24 original inputs')
(P / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
print('Prepared 24 exact original inputs plus independent metadata; request SHA', sha(P / 'request.json'))
