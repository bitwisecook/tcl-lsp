from pathlib import Path
import concurrent.futures
import hashlib
import json
import re
import shlex
import shutil
import subprocess
import time

repo = Path('/workspace/tcl-lsp')
request = Path('/tmp/sourceperf-encoding-read-order-request643d')
output = Path('/workspace/.proofs/root-native-source-read-order646')
assert not output.exists()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(request / 'manifest.json') == '0c6cc5e902f02143f48f3e4d424c3dadc8dd34a345989a55cd54aa6ec7d7bc2c'
manifest = json.loads((request / 'manifest.json').read_text())
for row in manifest['files']:
    assert sha(request / row['path']) == row['sha256'], row['path']
cases = json.loads((request / 'request.json').read_text())
output.mkdir()
for row in manifest['files']:
    shutil.copyfile(request / row['path'], output / row['path'])
shutil.copyfile(request / 'manifest.json', output / 'input-manifest.json')
shutil.copyfile(Path(__file__), output / 'collector.py')
existing = output / 'source-existing.tcl'
missing = output / 'source-guaranteed-absent.tcl'
existing.write_bytes(cases['source_content_ascii'].encode('ascii'))
assert not missing.exists()
directory = output / 'source-owned-empty-directory'
directory.mkdir()
version_source = output / 'version.tcl'
version_source.write_bytes(b'puts [info patchlevel]\n')
compiler = Path(shutil.which('cc')).resolve()
environment = {'PATH': '/usr/bin:/bin', 'LC_ALL': 'C', 'LANG': 'C'}

def execute(folder, name, argv, env, budget):
    started = time.monotonic()
    timed_out = False
    try:
        process = subprocess.run(argv, cwd=folder, env=env, capture_output=True, timeout=budget)
        code, out, err = process.returncode, process.stdout, process.stderr
    except subprocess.TimeoutExpired as e:
        timed_out, code, out, err = True, None, e.stdout or b'', e.stderr or b''
    stdout, stderr = folder / (name + '.stdout'), folder / (name + '.stderr')
    stdout.write_bytes(out)
    stderr.write_bytes(err)
    return {'argv': argv, 'cwd': str(folder), 'environment': env, 'timeout_seconds': budget,
            'timed_out': timed_out, 'exit': code, 'seconds': time.monotonic() - started,
            'stdout': str(stdout), 'stdout_sha256': sha(stdout),
            'stderr': str(stderr), 'stderr_sha256': sha(stderr)}

record = execute(output, 'compiler-version', [str(compiler), '--version'], environment, 15)
record.update({'compiler': str(compiler), 'compiler_sha256': sha(compiler)})
(output / 'compiler.json').write_text(json.dumps(record, indent=2) + '\n')

def provider(name):
    folder = output / name
    folder.mkdir()
    env = dict(environment)
    if name == 'Jim':
        root = Path('/workspace/.proofs/native-providers/jimtcl')
        library, shell = root / 'libjim.a', root / 'jimsh'
        includes, defines = [root], ['-DJIM_PROBE']
        link_flags = ['-lm', '-ldl', '-lssl', '-lcrypto', '-lz']
        sources = [p for p in root.iterdir() if p.is_file() and p.suffix in {'.c', '.h'}]
        sources += [root / 'config.log', root / 'Makefile']
        revision = execute(root, 'unused', ['git', 'rev-parse', 'HEAD'], env, 15) if False else None
        revision = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=root, env=env, capture_output=True)
        (folder / 'provider-revision.stdout').write_bytes(revision.stdout)
        (folder / 'provider-revision.stderr').write_bytes(revision.stderr)
        revision_record = {'argv': ['git', 'rev-parse', 'HEAD'], 'cwd': str(root), 'exit': revision.returncode,
                           'stdout_sha256': sha(folder / 'provider-revision.stdout'),
                           'stderr_sha256': sha(folder / 'provider-revision.stderr')}
        assert revision.returncode == 0 and revision.stdout.strip() == b'5bac7c99ad65864c87da513e22e2f01703fa4e03'
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
        sources = [p for d in ('generic', 'unix') for p in (root / d).iterdir()
                   if p.is_file() and p.suffix in {'.c', '.h'}]
        sources += [config, root / 'unix/Makefile']
        sources += [p for p in (root / 'library').rglob('*') if p.is_file()]
        revision_record = None
    inputs = [library, shell, compiler, version_source, existing, output / 'input-manifest.json', output / 'collector.py',
              *[output / x['path'] for x in manifest['files']], *sources]
    pins = {str(p): sha(p) for p in inputs}
    executable = folder / 'probe.elf'
    argv = [str(compiler), '-std=c99', '-O0', *defines, *['-I' + str(p) for p in includes],
            str(output / 'probe.c'), str(library), *link_flags, '-o', str(executable)]
    compilation = execute(folder, 'compile', argv, env, 60)
    version = execute(folder, 'cli-version', [str(shell), str(version_source)], env, 15)
    run = None
    blocks = []
    if compilation['exit'] == 0 and not compilation['timed_out']:
        executable_sha = sha(executable)
        assert sha(existing) == pins[str(existing)] and not missing.exists()
        run = execute(folder, 'execute', [str(executable), str(existing), str(missing), str(directory)], env, 30)
        run.update({'executable_sha256_before': executable_sha, 'executable_sha256_after': sha(executable)})
        raw = Path(run['stdout']).read_bytes()
        for data in raw.split(b'CASE|')[1:]:
            lines = data.splitlines()
            label, worker = lines[0].decode('ascii').split('|')
            entries = [line.decode('ascii').split('|') for line in lines[1:]]
            blocks.append({'label': label, 'worker': int(worker), 'lines': entries})
        (folder / 'observations.json').write_text(json.dumps(blocks, indent=2) + '\n')
    unchanged = all(sha(Path(p)) == h for p, h in pins.items())
    scope = ('One actual process per provider, 8 freshly created and initialized interpreters in that process. '
             'C system encoding explicitly reset to utf-8 before each case; raw names made by counted public '
             'String constructors, explicit argv and observer result/errorCode/effect holds. This observes '
             'completion bytes and mutation/effect only, not private callback chronology/refcount neutrality '
             'or Rust/BIG-IP outcomes; the retained libraries identify their actual build bytes, not guessed build commands.')
    receipt = {'proof_id': 'naming.encoding.original-byte-name-lookup-and-source-order', 'provider': name,
               'compile': compilation, 'cli_version': version, 'revision': revision_record,
               'execute': run, 'observations_count': len(blocks), 'inputs_before': pins,
               'all_after_pins_match': unchanged, 'source_file_sha256': sha(existing),
               'missing_file_absent_before_and_after': not missing.exists(), 'owned_directory': str(directory), 'owned_directory_remains_empty': directory.is_dir() and not list(directory.iterdir()), 'scope': scope + ' Directory open/read success stage is not assumed; exact full result/errorCode bytes and pinned original source must establish it.'}
    (folder / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    summary = {'provider': name, 'compile_exit': compilation['exit'], 'cli_version_exit': version['exit'],
               'process_exit': run['exit'] if run else None, 'observations_count': len(blocks),
               'all_after_pins_match': unchanged, 'receipt_sha256': sha(folder / 'receipt.json')}
    print(json.dumps(summary), flush=True)
    return summary

with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
    results = list(pool.map(provider, ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0', 'Jim']))
(output / 'results.json').write_text(json.dumps({'proof_id': 'naming.encoding.original-byte-name-lookup-and-source-order',
    'request_manifest_sha256': sha(output / 'input-manifest.json'), 'providers': results}, indent=2) + '\n')
assert all(r['compile_exit'] == 0 and r['cli_version_exit'] == 0 and r['process_exit'] == 0 and
           r['observations_count'] == 8 and r['all_after_pins_match'] for r in results)
