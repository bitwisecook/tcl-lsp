from pathlib import Path
import concurrent.futures
import hashlib
import json
import os
import re
import shlex
import shutil
import subprocess
import time

repo = Path('/workspace/tcl-lsp')
request = Path('/tmp/index-native-stride644')
output = Path('/workspace/.proofs/root-native-list-stride644')
assert not output.exists()
output.mkdir()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(request / 'manifest.json') == 'fdeb2fa97092565cc22b5587028586c0ed50753fd28586620983f052aa0b7ef4'
manifest = json.loads((request / 'manifest.json').read_text())
for item in manifest['files']:
    assert sha(request / item['path']) == item['sha256'] and (request / item['path']).stat().st_size == item['bytes']
spec = json.loads((request / 'request.json').read_text())
proof_id = 'naming.list.original-stride-primitive-and-results'
assert sha(request / 'probe.c') == spec['source_sha256']
shutil.copyfile(request / 'request.json', output / 'request.json')
shutil.copyfile(request / 'probe.c', output / 'probe.c')
version_source = output / 'version.tcl'
version_source.write_bytes(b'puts [info patchlevel]\n')
compiler = Path(shutil.which('cc')).resolve()
environment = {'PATH': '/usr/bin:/bin', 'LC_ALL': 'C', 'LANG': 'C'}

def execute(folder, name, argv, env, budget):
    started = time.monotonic()
    timed_out = False
    try:
        process = subprocess.run(argv, cwd=folder, env=env,
                                 capture_output=True, timeout=budget)
        code, out, err = process.returncode, process.stdout, process.stderr
    except subprocess.TimeoutExpired as e:
        timed_out, code, out, err = True, None, e.stdout or b'', e.stderr or b''
    stdout, stderr = folder / (name + '.stdout'), folder / (name + '.stderr')
    stdout.write_bytes(out)
    stderr.write_bytes(err)
    return {'argv': argv, 'cwd': str(folder), 'environment': env,
            'timeout_seconds': budget, 'timed_out': timed_out,
            'exit': code, 'seconds': time.monotonic() - started,
            'stdout': str(stdout), 'stdout_sha256': sha(stdout),
            'stderr': str(stderr), 'stderr_sha256': sha(stderr)}

compiler_record = execute(output, 'compiler-version', [str(compiler), '--version'], environment, 15)
compiler_record.update({'compiler': str(compiler), 'compiler_sha256': sha(compiler)})
(output / 'compiler.json').write_text(json.dumps(compiler_record, indent=2) + '\n')

def provider(name):
    folder = output / name
    folder.mkdir()
    env = dict(environment)
    if name == 'Jim':
        root = Path('/workspace/.proofs/native-providers/jimtcl')
        library, shell = root / 'libjim.a', root / 'jimsh'
        includes, defines = [root], ['-DPROBE_JIM']
        link_flags = ['-lm', '-ldl', '-lssl', '-lcrypto', '-lz']
        sources = [p for p in root.iterdir() if p.is_file() and p.suffix in {'.c', '.h'}]
        sources += [root / 'config.log', root / 'Makefile']
        revision = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=root,
                                  capture_output=True, env=env)
        (folder / 'provider-revision.stdout').write_bytes(revision.stdout)
        (folder / 'provider-revision.stderr').write_bytes(revision.stderr)
        revision_record = {'argv': ['git', 'rev-parse', 'HEAD'], 'cwd': str(root),
                           'exit': revision.returncode,
                           'stdout_sha256': sha(folder / 'provider-revision.stdout'),
                           'stderr_sha256': sha(folder / 'provider-revision.stderr')}
    else:
        root = repo / 'tmp' / ('tcl' + name)
        library = root / 'unix' / ('libtcl' + '.'.join(name.split('.')[:2]) + '.a')
        shell = root / 'unix/tclsh'
        includes, defines = [root / 'generic', root / 'unix'], []
        config = root / 'unix/tclConfig.sh'
        match = re.search(r"^TCL_LIBS='([^']*)'", config.read_text(), re.M)
        assert match
        link_flags = shlex.split(match.group(1))
        env['TCL_LIBRARY'] = str(root / 'library')
        sources = [p for p in (root / 'generic').iterdir() if p.is_file() and p.suffix in {'.c', '.h'}]
        sources += [p for p in (root / 'unix').iterdir() if p.is_file() and p.suffix in {'.c', '.h'}]
        sources += [config, root / 'unix/Makefile']
        sources += [p for p in (root / 'library').rglob('*') if p.is_file()]
        revision_record = None
    pins = {str(p): sha(p) for p in [library, shell, compiler, output / 'probe.c',
                                   output / 'request.json', version_source, *sources]}
    executable = folder / 'probe.elf'
    argv = [str(compiler), '-std=c99', '-O0', *defines,
            *['-I' + str(p) for p in includes], str(output / 'probe.c'),
            str(library), *link_flags, '-o', str(executable)]
    compilation = execute(folder, 'compile', argv, env, 60)
    version = execute(folder, 'cli-version', [str(shell), str(version_source)], env, 15)
    rows = []
    if compilation['exit'] == 0 and not compilation['timed_out']:
        executable_sha = sha(executable)
        for case in range(spec['case_count']):
            child = folder / ('case-%02d' % case)
            child.mkdir()
            assert sha(executable) == executable_sha
            result = execute(child, 'execute', [str(executable), str(case)], env, 30)
            result.update({'case': case, 'executable_sha256_before': executable_sha,
                           'executable_sha256_after': sha(executable)})
            (child / 'receipt.json').write_text(json.dumps(result, indent=2) + '\n')
            rows.append(result)
    unchanged = all(sha(Path(p)) == h for p, h in pins.items())
    record = {'proof_id': proof_id, 'provider': name,
              'request_sha256': sha(output / 'request.json'),
              'probe_sha256': sha(output / 'probe.c'), 'compile': compilation,
              'cli_version': version, 'revision': revision_record, 'cases': rows,
              'inputs_before': pins, 'all_after_pins_match': unchanged,
              'scope': spec['scope'],
              'limits': 'Fresh process public EvalObjv original counted stride String and empty native List only. Exactly one explicit genuine argument-owner role per object and no extra independent observer reference. Unsupported command options remain Guest outcomes. No Tcl source decoding, Rust assertion credit, private callback/refcount chronology, complete system linker closure or inferred historical library build invocation. Native libraries identified by actual retained bytes/configuration and independently observed version.'}
    (folder / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
    summary = {'provider': name, 'compile_exit': compilation['exit'],
               'cli_version_exit': version['exit'], 'executions': len(rows),
               'process_exits': [r['exit'] for r in rows],
               'all_after_pins_match': unchanged,
               'receipt_sha256': sha(folder / 'receipt.json')}
    print(json.dumps(summary), flush=True)
    return summary

with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
    results = list(pool.map(provider, ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0', 'Jim']))
(output / 'results.json').write_text(json.dumps({'proof_id': proof_id,
    'problem': spec['question'], 'request_sha256': sha(output / 'request.json'),
    'source_sha256': sha(output / 'probe.c'), 'providers': results,
    'scope': spec['scope']}, indent=2) + '\n')
assert all(r['compile_exit'] == 0 and r['cli_version_exit'] == 0 and
           r['executions'] == spec['case_count'] and set(r['process_exits']) == {0} and
           r['all_after_pins_match'] for r in results)
