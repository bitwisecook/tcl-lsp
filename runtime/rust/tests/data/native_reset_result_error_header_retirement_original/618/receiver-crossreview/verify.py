import hashlib
import json
import pathlib
import re
from collections import Counter

BASE = pathlib.Path('/workspace/.proofs/native-reset-result-error-header-retirement618')
OUT = pathlib.Path('/tmp/receiver-native-reset-result618-crossreview')

def sha(path):
    h = hashlib.sha256()
    with pathlib.Path(path).open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()

def read(path):
    return json.loads(pathlib.Path(path).read_bytes())

def save(name, value):
    path = OUT / name
    path.write_text(json.dumps(value, indent=2) + '\n')
    return {'path': str(path), 'sha256': sha(path), 'bytes': path.stat().st_size}

def exact_identity(record):
    p = pathlib.Path(record['path'])
    st = p.stat()
    actual = {'path': str(p), 'resolved_path': str(p.resolve()), 'sha256': sha(p),
              'size': st.st_size, 'device': st.st_dev, 'inode': st.st_ino,
              'mtime_ns': st.st_mtime_ns, 'ctime_ns': st.st_ctime_ns}
    assert all(actual[k] == v for k, v in record.items() if k in actual), (str(p), 'identity changed')
    return actual

request = read(BASE / 'request.json')
manifest = read(BASE / 'manifest.json')
closure = read(BASE / 'closure.json')
rd = sha(BASE / 'request.json')
assert rd == manifest['request_sha256'] == closure['request_sha256']
assert closure['verification_status'] == 'verified' and closure['harness_refusal'] is None
assert closure['required_inputs_unchanged'] and closure['all_capture_processes_closed']
assert len(closure['records']) == 16 and closure['completed_verified_provider_executions'] == 5
for file in manifest['files']:
    assert sha(file['path']) == file['sha256']
    assert pathlib.Path(file['path']).stat().st_size == file['bytes']
for path, digest in request['required_sha256'].items():
    assert sha(path) == digest, path
print('Current manifested files and all', len(request['required_sha256']), 'declared input digests verified.', flush=True)
source_joins = read(BASE / 'source-windows.json')
for join in source_joins:
    raw = pathlib.Path(join['source']).read_bytes()
    assert sha(join['source']) == join['source_sha256']
    excerpt = b'\n'.join(raw.split(b'\n')[join['physical_LF_first'] - 1:join['physical_LF_last']]) + b'\n'
    assert excerpt == pathlib.Path(join['window']).read_bytes()
    assert sha(join['window']) == join['window_sha256']

records = []
for record in closure['records']:
    p = pathlib.Path(record['path'])
    assert sha(p) == record['sha256']
    receipt = read(p)
    assert receipt['provider'] == record['provider'] and receipt['channel'] == record['channel']
    assert receipt['status'] == 'closed' and receipt['returncode'] == 0 and not receipt['timeout']
    assert receipt['request_sha256'] == rd and receipt['source_sha256'] == request['probe_sha256']
    for stream in ['stdout', 'stderr']:
        assert sha(p.parent / (receipt['channel'] + '.' + stream)) == receipt[stream + '_sha256']
    records.append(dict(record, returncode=receipt['returncode'], timeout=receipt['timeout'],
                        seconds=receipt['seconds'], command=receipt['command'], cwd=receipt['cwd'],
                        stdout_sha256=receipt['stdout_sha256'], stderr_sha256=receipt['stderr_sha256']))
compiler = read(BASE / 'compiler-version.receipt.json')
assert compiler['command'] == [request['compiler'], '--version'] and compiler['cwd'] == str(BASE)
providers = []
all_cases = []
totals = Counter()
modern_normalised = []
for build in request['provider_builds']:
    version = build['provider']
    d = BASE / 'providers' / version
    cr, dr, er = [read(d / (ch + '.receipt.json')) for ch in ['compile', 'dependencies', 'execute']]
    cv, dv, ev = [read(d / (ch + '.verification.json')) for ch in ['compile', 'dependencies', 'execute']]
    assert cr['command'] == build['command'] and cr['cwd'] == build['compile_cwd']
    assert cr['selected_environment'] == dict(compiler['selected_environment'], **build['environment'])
    assert er['selected_environment'] == cr['selected_environment']
    assert dr['selected_environment'] == compiler['selected_environment']
    assert not cv['output_existed_before']
    image = exact_identity(cv['executable_produced'])
    assert pathlib.Path(image['path']).read_bytes()[:4] == b'\x7fELF'
    assert er['command'] == [image['path']] and er['cwd'] == str(BASE)
    assert dr['command'] == ['/usr/bin/ldd', image['path']] and dr['cwd'] == str(BASE)
    for ch, verification in [('compile', cv), ('dependencies', dv), ('execute', ev)]:
        assert verification['status'] == 'verified' and verification['request_sha256'] == rd
        assert verification['process_receipt'] == str(d / (ch + '.receipt.json'))
        assert verification['process_receipt_sha256'] == sha(d / (ch + '.receipt.json'))
    exact_identity(cv['link_map'])
    exact_identity(cv['header_dependency_file'])
    header_paths = []
    dependencies = (d / 'compile.d').read_text().replace('\\\n', '').split(':', 1)[1].split()
    for name in dependencies:
        p = pathlib.Path(name)
        header_paths.append(str((p if p.is_absolute() else pathlib.Path(build['compile_cwd']) / p).resolve()))
    assert header_paths == [h['resolved_path'] for h in cv['actual_header_and_source_dependencies']]
    for h in cv['actual_header_and_source_dependencies']:
        exact_identity(h)
        assert h['sha256'] == request['required_sha256'][h['resolved_path']]
    link_paths = []
    for line in (d / 'link.map').read_text().split('\n'):
        if line.startswith('LOAD '):
            p = pathlib.Path(line[5:])
            link_paths.append(str(p if p.is_absolute() else pathlib.Path(build['compile_cwd']) / p))
    assert link_paths == [h['path'] for h in cv['actual_link_inputs']]
    ephemeral = []
    for h in cv['actual_link_inputs']:
        if h['status'] == 'compiler-produced-ephemeral-object-not-retained':
            assert h['sha256'] is None and h['path'].startswith('/tmp/') and h['path'].endswith('.o')
            ephemeral.append(h['path'])
        else:
            exact_identity(h)
            assert h['sha256'] == request['required_sha256'][h['resolved_path']]
    dyn_paths = []
    for line in (d / 'dependencies.stdout').read_text().split('\n'):
        dyn_paths.extend(re.findall(r'(?<!\S)(/[^\s()]+)', line))
    assert dyn_paths == [h['path'] for h in dv['actual_dynamic_dependencies']]
    for h in dv['actual_dynamic_dependencies']:
        exact_identity(h)
        assert h['sha256'] == request['required_sha256'][h['resolved_path']]
    assert dv['executable_before'] == dv['executable_after'] == image
    assert ev['executable_before'] == ev['executable_after'] == ev['own_successful_compile_producer'] == image
    assert ev['dependencies_before'] == ev['dependencies_after'] == dv['actual_dynamic_dependencies']
    assert ev['executable_unchanged'] and ev['closed_cases'] == 5 and ev['closed_marker']
    assert ev['compile_receipt_sha256'] == sha(d / 'compile.receipt.json')
    assert ev['compile_verification_sha256'] == sha(d / 'compile.verification.json')
    assert ev['dependency_receipt_sha256'] == sha(d / 'dependencies.receipt.json')
    assert ev['dependency_verification_sha256'] == sha(d / 'dependencies.verification.json')
    lines = (d / 'execute.stdout').read_text().split('\n')
    assert lines[0] == 'VERSION|' + version + '|release_type=2|header_major=' + version.split('.')[0] + '|header_minor=' + version.split('.')[1]
    assert ev['actual_version_rows'] == [lines[0]]
    assert [l for l in lines if l.startswith('CLOSED_CASE|')] == ['CLOSED_CASE|' + str(c) for c in range(5)]
    assert lines[-2] == 'CLOSED_ORIGINAL_RESET_RESULT_ERROR_HEADER_RETIREMENT'
    is84 = version == '8.4.20'
    counts = Counter()
    norm = []
    for c in range(5):
        selected = []
        for line_number, line in enumerate(lines, 1):
            parts = line.split('|')
            if parts[0] in ['CASE', 'SETUP', 'ROW', 'EVENT', 'GETTER'] and parts[1] == str(c):
                values = dict(part.split('=', 1) for part in parts if '=' in part)
                selected.append(dict(kind=parts[0], line=line_number, values=values))
                counts[parts[0]] += 1
        rows = [s['values'] for s in selected if s['kind'] == 'ROW']
        events = [s['values'] for s in selected if s['kind'] == 'EVENT']
        getters = [s['values'] for s in selected if s['kind'] == 'GETTER']
        initial = next(s['values'] for s in selected if s['kind'] == 'CASE')
        assert initial['init_code'] == '0' and initial['selected_c84'] == str(int(is84))
        assert initial['private_info_fixture'] == str(int(not is84 and c != 0))
        before, after, final = rows[0], rows[1], rows[-1]
        assert before['stage'] == 'before_reset' and after['stage'] == 'after_reset_before_any_getter'
        assert final['stage'] == 'after_explicit_caller_releases'
        assert before['result'] == after['result'] == 'R' and before['result_header'] == after['result_header'] == before['R_header']
        assert before['R_refs'] == '1' and before['result_string'] == before['result_type_none'] == '0'
        assert after['result_string'] == after['result_type_none'] == '1' and after['result_length'] == '0'
        assert after['R_free_int_rep'] == '1' and after['R_refs'] == 'NA'
        assert all(row[r + '_duplicates'] == '0' for row in rows for r in ['R', 'C', 'I'])
        assert all(after[r + '_updates'] == '0' for r in ['R', 'C', 'I'])
        reset = [e for e in events if e['phase'] == 'actual_reset_result']
        assert reset[0]['kind'] == 'free_int_rep' and reset[0]['record'] == 'R' and reset[0]['passed_refs'] == '1'
        assert reset[0]['result_header'] == before['result_header']
        if is84:
            assert len(reset) == 1 and after['code'] == before['code'] and after['info'] == before['info']
            assert after['explicit'] == '0' and after['copy'] == '-1'
            assert after['code_trace'] == after['info_trace'] == '0'
            if c:
                assert before['explicit'] == '1' and after['C_refs'] == before['C_refs']
        else:
            assert after['code'] == after['info'] == 'NONE' and after['copy'] == '0'
            if c == 1:
                assert [(e['kind'], e['record'], e['passed_refs'], e['code'], e['info']) for e in reset] == [
                    ('free_int_rep', 'R', '1', 'C', 'I'), ('free_int_rep', 'C', '0', 'C', 'I'), ('free_int_rep', 'I', '0', 'NONE', 'I')]
            elif c == 3:
                assert before['copy'] == '1' and after['C_refs'] == after['I_refs'] == '2'
                assert [(e['kind'], e['code'], e['info'], e['copy']) for e in reset] == [
                    ('free_int_rep', 'C', 'I', '1'), ('global_code_write_trace', 'C', 'I', '1'), ('global_info_write_trace', 'NONE', 'I', '1')]
                assert after['code_trace'] == after['info_trace'] == '1'
            else:
                assert len(reset) == 1
                if c in [2, 4]: assert after['C_refs'] == '1'
        assert len(getters) == (2 if c in [2, 3] else 1 if c == 4 else 0)
        assert [g['header'] for g in getters] == (['C', 'I'] if c in [2, 3] else ['C'] if c == 4 else [])
        for g in getters:
            assert g['count'] == '14' and g['hex'] == request['raw_payload_hex']
        assert all(e['phase'] == 'explicit_caller_header_string_getter' for e in events if e['kind'] == 'update_string')
        if c == 4:
            assert before['code_header'] == before['info_header'] == before['C_header'] and before['C_refs'] == '3'
            assert before['I_header'] == '(nil)' and after['C_refs'] == ('3' if is84 else '1')
        if c in [2, 3]:
            assert before['C_refs'] == before['I_refs'] == '2'
        if is84 and c:
            assert final['C_refs'] == ('2' if c == 4 else '1') and final['C_free_int_rep'] == '0'
        if not is84 and c == 3:
            assert final['C_refs'] == final['I_refs'] == '1' and final['C_free_int_rep'] == final['I_free_int_rep'] == '0'
        normalised = [{k: v for k, v in s['values'].items() if 'header' not in k} for s in selected]
        norm.append(normalised)
        all_cases.append({'provider': version, 'case': c, 'raw_stdout_sha256': sha(d / 'execute.stdout'),
                          'input_role': request['cases'][c], 'observations': selected})
    if not is84:
        modern_normalised.append(norm)
    totals.update(counts)
    providers.append({'provider': version, 'executable': image,
                      'compile_receipt_sha256': sha(d / 'compile.receipt.json'),
                      'execute_receipt_sha256': sha(d / 'execute.receipt.json'),
                      'actual_header_dependency_count': len(header_paths),
                      'actual_link_input_count': len(link_paths),
                      'compiler_ephemeral_object_paths_not_digested': ephemeral,
                      'dynamic_dependency_count': len(dyn_paths), 'record_counts': dict(counts)})
assert all(m == modern_normalised[0] for m in modern_normalised[1:])
assert totals == Counter(CASE=25, SETUP=25, ROW=100, EVENT=78, GETTER=25)
OUT.mkdir(exist_ok=False)
case_file = save('case-observations.json', all_cases)
interpretation = {
    'question': request['question_id'], 'status': 'Independent bounded actual response cross-review; no agent execution',
    'original_request_sha256': rd, 'original_manifest_sha256': sha(BASE / 'manifest.json'),
    'original_closure_sha256': sha(BASE / 'closure.json'), 'probe_sha256': request['probe_sha256'],
    'audit': {'manifest_files': len(manifest['files']), 'current_required_input_digests': len(request['required_sha256']),
              'physical_LF_source_joins': len(source_joins), 'process_records': len(records), 'process_exit_codes': 'all 0',
              'fresh_own_compiler_images': 5, 'complete_cases': 25, 'record_counts': dict(totals),
              'receipt_and_stream_hashes_rejoined': True, 'actual_compile_argv_cwd_environment_matches_request': True,
              'actual_header_dependency_map_rejoined': True, 'actual_link_map_rejoined': True,
              'actual_runtime_dependency_output_rejoined': True, 'own_ELF_complete_identity_before_after_matches_current': True,
              'modern_pointer_free_observation_projection_equal': True},
    'providers': providers, 'process_receipts': records, 'source_joins': source_joins, 'case_observations': case_file,
    'measured_conclusions': [
        'Every case gives the actual result one unshared interpreter-owned role. Reset retires its internal representation with callback refcount1 while the current result still denotes that same original header, then retains the same current header with empty String and no type. The shared-result replacement branch is not exercised.',
        'C84 retains the original global code/info headers and native roles through ResetResult, clears the explicit error-code flag, and invokes no reset-time global write trace in the registered-trace case. Error headers stay lazy; true caller-held getters later return exactly14 counted bytes including NUL and ff.',
        'C85/C86/C90/C91 have identical measured pointer-free role projections. In the storage-only case the reset callbacks run result, code, info; code callback sees original code/info, info callback sees code NULL and original info. Both private slots are NULL at return. No String updater is reached.',
        'Modern distinct caller+private headers decrease from refs2 to refs1 at reset without error-header free. The separately labelled caller getters reach their updaters; subsequent caller release reaches code/info free at refcount0, with private slots already NULL.',
        'The explicit modern ERR_LEGACY_COPY input reaches global code then global info write traces while the corresponding original private field is still installed, clears both private fields and the flag, and preserves caller+compatibility-global refs2 without String conversion. Caller release leaves the measured globals role refs1; teardown callbacks are unobserved.',
        'The shared code/info input has the same original header in two independent storage roles plus one caller role, refs3. Modern reset releases two private roles leaving refs1; C84 leaves the two global roles and caller refs3. The sole caller getter acts on the retained same header; later caller release gives modern free at0 or C84 globals refs2.'
    ],
    'input_and_observation_limits': [
        'Modern lazy errorInfo is an explicitly prepared authentic private slot with its declared +1; these cases do not measure a public lazy-errorInfo producer.',
        'Modern ERR_LEGACY_COPY is an explicitly installed authentic flag fixture, not a claimed public producer. C84 info uses the actual public global variable setter and code uses actual Tcl_SetObjErrorCode.',
        'Pure free/update/global-trace observers invoke no interpreter worker or mutating reentry. There is no Host/ABA/first-cause, callback-reentry, Jim, BIG-IP, Runtime/VM assertion, whole-suite, or authority grant.',
        'Original-header refs/String are deliberately NA after any freeIntRep callback; that callback alone proves neither object death nor survival. Actual current result is observed through its independent live interpreter-owned role.',
        'Addresses are diagnostic original-role correspondence within each fresh case, not generational identity. Interpreter teardown has observation disabled and yields no measured callback ordering.',
        'Actual link maps retain one compiler-created ephemeral object path per producer without its bytes/digest. All other actual header/link/runtime inputs are required pinned originals. SDK configure/build replay is not available.',
        'Earlier failed614 and partial614b observations remain separate and untouched. No setter executable is reused or credited as this reset producer.'
    ]
}
file = save('interpretation.json', interpretation)
audit_script = {'path': __file__, 'sha256': sha(__file__), 'bytes': pathlib.Path(__file__).stat().st_size}
sealed = save('manifest.json', {'status': 'READY independent actual Native618 cross-review',
                               'question': request['question_id'], 'files': [file, case_file, audit_script],
                               'original_request_sha256': rd, 'original_closure_sha256': sha(BASE / 'closure.json')})
print(json.dumps({'manifest': sealed, 'interpretation': file, 'counts': dict(totals), 'providers': providers}, indent=2))
