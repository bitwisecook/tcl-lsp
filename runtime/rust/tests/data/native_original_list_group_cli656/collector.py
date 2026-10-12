from pathlib import Path
import concurrent.futures
import hashlib
import json
import re
import resource
import shutil
import subprocess
import time

REQUEST = Path('/workspace/.proofs/list-original-group-cli656')
OUT = Path('/workspace/.proofs/root-native-list-group-cli656')
PREVIOUS = Path('/workspace/.proofs/root-native-list-group-command651')
assert not OUT.exists()
OUT.mkdir()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(REQUEST / 'manifest.json') == '3303cbea3c9585c8320da71abf7e5bc737ba9a0997fcb9821daab78575fb5a6c'
manifest = json.loads((REQUEST / 'manifest.json').read_text())
for row in manifest['files']:
    assert sha(REQUEST / row['path']) == row['sha256']
    shutil.copyfile(REQUEST / row['path'], OUT / row['path'])
shutil.copyfile(REQUEST / 'manifest.json', OUT / 'input-manifest.json')
shutil.copyfile(Path(__file__), OUT / 'collector.py')
spec = json.loads((OUT / 'request.json').read_text())
version_driver = OUT / 'version.tcl'
version_driver.write_bytes(b'puts "PATCHLEVEL|[info patchlevel]"\nforeach key {library nameofexecutable loaded} {set code [catch [list info $key] value]; puts "INFO|$key|$code|$value"}\nset code [catch {encoding system} value]; puts "ENCODING|$code|$value"\n')
resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
base_env = {'PATH': '/usr/bin:/bin', 'LC_ALL': 'C', 'LANG': 'C', 'TCLLIBPATH': ''}

def run(folder, name, command, env, timeout=20):
    start = time.monotonic()
    timed_out = False
    try:
        r = subprocess.run(command, cwd=folder, env=env, capture_output=True, timeout=timeout)
        code, stdout, stderr = r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired as r:
        timed_out, code, stdout, stderr = True, None, r.stdout or b'', r.stderr or b''
    out, err = folder / (name + '.stdout'), folder / (name + '.stderr')
    out.write_bytes(stdout)
    err.write_bytes(stderr)
    return {'command': command, 'cwd': str(folder), 'explicit_environment': env,
            'timeout_seconds': timeout, 'timed_out': timed_out, 'exit': code,
            'seconds': time.monotonic() - start, 'stdout': str(out), 'stderr': str(err),
            'stdout_sha256': sha(out), 'stderr_sha256': sha(err)}

def provider(name):
    folder = OUT / name
    folder.mkdir()
    previous = json.loads((PREVIOUS / name / 'receipt.json').read_text())
    inputs = previous['inputs_before']
    root = Path('/workspace/.proofs/native-providers/jimtcl') if name == 'Jim' else Path('/workspace/tcl-lsp/tmp') / ('tcl' + name)
    shell = root / ('jimsh' if name == 'Jim' else 'unix/tclsh')
    library = root / ('libjim.a' if name == 'Jim' else 'unix/libtcl' + '.'.join(name.split('.')[:2]) + '.a')
    env = dict(base_env)
    if name != 'Jim':
        env['TCL_LIBRARY'] = str(root / 'library')
    sources = [Path(p) for p in inputs if Path(p).is_relative_to(root)]
    assert shell in sources and library in sources
    assert all(sha(p) == inputs[str(p)] for p in sources)
    version_elf = PREVIOUS / name / 'same-library-version/version.elf'
    version_receipt_path = PREVIOUS / name / 'same-library-version/receipt.json'
    assert version_elf.exists() and version_receipt_path.exists()
    version_receipt = json.loads(version_receipt_path.read_text())
    dynamic = run(folder, 'loader-diagnostic', ['/usr/bin/ldd', str(shell)], env)
    paths = {Path(p) for p in re.findall(r'(?:=>\s+|^\s*)(/[^\s]+)', Path(dynamic['stdout']).read_text(), re.M) if Path(p).is_file()}
    paths.add(Path('/usr/bin/ldd'))
    pinned = {str(p): sha(p) for p in [*sources, *paths, version_elf, version_receipt_path,
                  version_driver, OUT / 'probe.tcl', OUT / 'request.json', OUT / 'input-manifest.json', OUT / 'collector.py']}
    version = run(folder, 'cli-version', [str(shell), str(version_driver)], env)
    static_version = run(folder, 'same-library-version', [str(version_elf)], env)
    revision = run(folder, 'provider-revision', ['/usr/bin/git', '-C', str(root), 'rev-parse', 'HEAD'], env) if name == 'Jim' else None
    cases = []
    for number in range(11):
        child = folder / ('case-%02d' % number)
        child.mkdir()
        record = run(child, 'execute', [str(shell), str(OUT / 'probe.tcl'), str(number)], env)
        fields = {}
        for line in Path(record['stdout']).read_bytes().splitlines():
            parts = line.split(b'\t')
            if len(parts) >= 2:
                key = parts[0].decode('ascii')
                assert key not in fields
                fields[key] = [p.decode('ascii') for p in parts[1:]]
        record['fields'] = fields
        record['case'] = number
        record['complete_projection'] = set(fields) == {'case','script-hex','guest-code','result-hex','error-code-present','error-code-hex','error-info-present','error-info-hex','callback-count'}
        if record['complete_projection']:
            assert fields['case'] == [str(number), spec['cases'][number][1]]
            for key in ['script-hex','result-hex','error-code-hex','error-info-hex']:
                assert len(fields[key]) == 1
                bytes.fromhex(fields[key][0])
        (child / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
        cases.append(record)
    unchanged = all(sha(Path(p)) == h for p, h in pinned.items())
    record = {'provider': name, 'question': spec['question'], 'cases': cases,
              'cli_version': version, 'same_library_version': static_version,
              'earlier_same_library_build_receipt': str(version_receipt_path),
              'earlier_same_library_build_receipt_sha256': sha(version_receipt_path),
              'revision': revision, 'loader_diagnostic': dynamic,
              'inputs_before': pinned, 'all_after_pins_match': unchanged,
              'scope': spec['scope'], 'limits': 'Fresh initialized CLI source/public fields only. Loader output identifies diagnostic resolved paths, not a complete linker/runtime closure. Earlier same-library diagnostic is freshly executed with exact original ELF, receipt and selected SDK hashes. No private getter/header/role/sort chronology, Native build command, Rust or BIG-IP result is inferred.'}
    (folder / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
    summary = {'provider': name, 'cli_version_exit': version['exit'],
               'same_library_version_exit': static_version['exit'],
               'case_count': len(cases), 'process_exits': [x['exit'] for x in cases],
               'complete_projection_count': sum(x['complete_projection'] for x in cases),
               'all_after_pins_match': unchanged, 'receipt_sha256': sha(folder / 'receipt.json')}
    print(json.dumps(summary), flush=True)
    return summary

with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
    summaries = list(pool.map(provider, spec['providers']))
(OUT / 'results.json').write_text(json.dumps({'question': spec['question'], 'request_sha256': sha(OUT / 'request.json'), 'source_sha256': sha(OUT / 'probe.tcl'), 'providers': summaries, 'scope': spec['scope']}, indent=2) + '\n')
assert all(x['cli_version_exit'] == x['same_library_version_exit'] == 0 and x['all_after_pins_match'] and x['case_count'] == x['complete_projection_count'] == 11 and set(x['process_exits']) == {0} for x in summaries)
