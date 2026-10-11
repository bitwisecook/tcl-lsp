from pathlib import Path
import ast, hashlib, json, re, shlex

BASE = Path(__file__).resolve().parent
ROOT = Path('/workspace/tcl-lsp')
ORIGIN = Path('/workspace/.proofs/native-variable-introspection-order505b/request.json')
digest = lambda data: hashlib.sha256(data).hexdigest()
original_raw = ORIGIN.read_bytes()
original = json.loads(original_raw)
versions = ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0']

# Reuse the existing strict compiler/header/link/runtime capture harness. Only
# this request's five providers, four case markers and final marker differ.
launcher_source = Path(original['launcher_file'])
launcher = launcher_source.read_text()
assert digest(launcher_source.read_bytes()) == original['launcher_sha256']
assert 'len(executions) == 6' in launcher and "item['closed_cases'] == 18" in launcher
launcher = launcher.replace('len(executions) == 6', 'len(executions) == 5')
launcher = launcher.replace("item['closed_cases'] == 18", "item['closed_cases'] == 4")
launcher = launcher.replace('CLOSED_ORIGINAL_VARIABLE_INTROSPECTION_ORDER', 'CLOSED_ORIGINAL_EXPLICIT_ERROR_CODE_HEADER_SETTER')
launcher = launcher.replace("if b'|version|' in line", "if line.startswith(b'VERSION|')")
launcher = launcher.replace('Original variable/introspection chronology closure:', 'Original explicit error-code header API closure:')
launcher = launcher.replace("'Jim unsupported observer/namespace purposes stay explicit.'", "'Jim has no corresponding Tcl API worker and is not executed for this question.'")
ast.parse(launcher)
(BASE / 'launch.py').write_text(launcher)

sdk_roots = [str(ROOT / ('tmp/tcl' + version)) + '/' for version in versions]
pins = {path: value for path, value in original['required_sha256'].items()
    if path.startswith(('/usr/', '/lib/', '/lib64/')) or any(path.startswith(prefix) for prefix in sdk_roots)}
builds, configurations = [], []
for version in versions:
    old = next(provider for provider in original['provider_builds'] if provider['provider'] == version)
    sdk = ROOT / ('tmp/tcl' + version)
    config = sdk / 'unix/tclConfig.sh'
    config_raw = config.read_bytes()
    if str(config) in pins:
        assert digest(config_raw) == pins[str(config)]
    pins[str(config)] = digest(config_raw)
    match = re.search(r"(?m)^TCL_DEFS='([^']*)'$", config_raw.decode())
    assert match, config
    definitions = shlex.split(match[1])
    assert definitions and all(flag.startswith('-D') and '$' not in flag for flag in definitions)
    directory = BASE / 'providers' / version
    library = next(argument for argument in old['command'] if argument.startswith(str(sdk)) and argument.endswith('.a'))
    assert str(Path(library)) in pins
    assert digest(Path(library).read_bytes()) == pins[str(Path(library))]
    command = [original['compiler'], '-MD', '-MF', str(directory / 'compile.d'), '-std=c99', '-O0',
        *definitions, '-I' + str(sdk / 'generic'), '-I' + str(sdk / 'unix'),
        '-I' + str(sdk / 'compat'), str(BASE / 'probe.c'), library, '-lm', '-ldl', '-lpthread', '-lz',
        '-Wl,-Map,' + str(directory / 'link.map'), '-o', str(directory / 'probe.elf')]
    builds.append(dict(provider=version, command=command, environment=old['environment']))
    configurations.append(dict(provider=version, original_config=str(config), config_sha256=digest(config_raw),
        literal_TCL_DEFS=match[1], selected_compiler_definitions=definitions,
        source_library_equivalence='No configure/build replay receipt is attached. The actual own compilation, dependency closure and linked static library bytes are independently captured.'))

join_path = Path('/tmp/publication-source-contracts614615/files/runtime/rust/tests/data/original_explicit_error_code_header_setter_source/source-joins.json')
joins = json.loads(join_path.read_bytes())
for join in joins:
    source = Path(join['original_path'])
    raw = source.read_bytes()
    assert digest(raw) == join['original_sha256']
    snippets = raw.split(b'\n')
    lines = [item + b'\n' for item in snippets[:-1]] + ([snippets[-1]] if snippets[-1] else [])
    excerpt = b''.join(lines[join['first_physical_lf_line'] - 1:join['last_physical_lf_line']])
    assert digest(excerpt) == join['excerpt_sha256']
    selected = BASE / 'source-windows' / (join['version'] + '-Tcl_SetObjErrorCode.c')
    selected.parent.mkdir(exist_ok=True)
    selected.write_bytes(excerpt)
    join['request_excerpt'] = str(selected)
    join['request_excerpt_sha256'] = digest(excerpt)
    pins[str(source)] = digest(raw)
    pins[str(selected)] = digest(excerpt)
source_associations = (json.dumps(joins, indent=2) + '\n').encode()
(BASE / 'source-windows.json').write_bytes(source_associations)
for name in ['probe.c', 'launch.py', 'prepare-request.py', 'source-windows.json']:
    path = BASE / name
    pins[str(path)] = digest(path.read_bytes())
request = dict(question_id='naming.error.original-explicit-error-code-header-setter',
    document='docs/design/analysis/name-resolution-proofs/error-original-explicit-error-code-header-setter.md',
    fixture_root='runtime/rust/tests/data/native_explicit_error_code_header_setter_original',
    status='prepared-not-compiled-or-executed; Root alone launches after independent source/protocol review',
    problem_statement='Does the actual public Tcl_SetObjErrorCode retain the same original lazy header without a setter String getter, and which selected old-header free/global callback/flag stages are reached before a separately requested String getter or public error return?',
    compiler=original['compiler'], probe_file=str(BASE / 'probe.c'), probe_sha256=digest((BASE / 'probe.c').read_bytes()),
    launcher_file=str(BASE / 'launch.py'), launcher_sha256=digest((BASE / 'launch.py').read_bytes()),
    providers=versions, provider_builds=builds, required_sha256=pins,
    independent_producer_configuration=configurations,
    reused_admission_origin=dict(path=str(ORIGIN), sha256=digest(original_raw),
        launcher_source=str(launcher_source), launcher_source_sha256=original['launcher_sha256'],
        purpose='Exact existing SDK/oracle strict compiler/header/link/runtime pin and capture machinery only. No prior observation, executable or assertion is reused or spliced into this request.'),
    cases=[
        dict(id=0, purpose='Actual setter with the new lazy caller-held header; no observed old custom header. Record selected global/private pointer and updater/refcount/result-header facts before setter, after return and after a separate explicit public String getter.'),
        dict(id=1, purpose='Install old original lazy header, drop its caller hold, then set a distinct new original header. The pure free hook records the selected current storage pointer/flag before returning; no interpreter worker or String getter is called by the hook.'),
        dict(id=2, purpose='Same held old/new headers plus a real public global errorCode write trace. C8.4 uses actual Tcl_ResetResult to clear its flag while retaining the global cell. Modern private setters and an independently registered global variable trace remain distinct; no modern global callback is assumed.'),
        dict(id=3, purpose='An actual installed original object command sets its result and lazy explicit code and returns TCL_ERROR. Callback-internal setter rows, public Tcl_EvalObjv return and later explicit header String getter remain separate; an Eval/error-context read is not attributed to the setter alone.')],
    observers=dict(C84_storage='Read the already-existing global errorCode scalar directly through the original global namespace hash table; only Tcl_FindHashEntry/Tcl_GetHashValue and scalar/undefined flags are used. No GetVar/SetVar, missing-shell creation, script, String getter or result rendering occurs inside extension free/update/trace hooks.',
        modern_storage='Read the selected original private Interp.errorCode pointer from the original tclInt.h; compiler definitions are copied literally from the selected retained tclConfig.sh.',
        row='Header/counter metadata plus the explicit public Tcl_GetObjResult header observer; no String getter in ROW. Pointer values are within-case diagnostics only, without generation, frame or cross-provider identity claims.',
        callbacks='Pure counters and original current-storage/header/flag metadata. All stdout is unbuffered. Direct hooks issue no interpreter/table mutation and no error formatter.',
        presentation='Only the separately labelled public Tcl_GetStringFromObj phase requests counted bytes. A later explicit caller-header getter is not evidence that a previously selected worker reached that getter.',
        teardown='Metadata observers are disabled before original final interpreter/header release; final teardown chronology is outside this question.'),
    raw_payload_hex=bytes([79,82,73,71,73,78,65,76,0,255,32,67,79,68,69]).hex(),
    jim=dict(status='not-tested for this Tcl API purpose', reason='No corresponding Tcl_SetObjErrorCode Jim C API worker is selected. No simulated Jim execution or inferred Guest rejection is supplied.'),
    bigip='not-tested',
    scope='Same immutable original C source and four declared cases independently compile/execute for each of five C releases. Actual process success, observer rows, selected API getter/count/old-free facts and public Eval error presentation remain separate. No Rust callback parity, safe reentrant lifetime, arbitrary error-worker chronology or performance grant follows.')
raw = (json.dumps(request, indent=2) + '\n').encode()
(BASE / 'request.json').write_bytes(raw)
manifest = dict(request=str(BASE/'request.json'), request_sha256=digest(raw),
    probe_sha256=request['probe_sha256'], launcher_sha256=request['launcher_sha256'],
    required_pin_count=len(pins), source_association_sha256=digest(source_associations),
    source_only_preparation=True, compiler_native_or_assertion_launches=0,
    independent_review_required_before_Root_launch=True)
(BASE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(manifest))
