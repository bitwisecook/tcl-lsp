import pathlib, subprocess, hashlib, json, os

repo = pathlib.Path('/workspace/tcl-lsp')
probe = pathlib.Path('/workspace/.proofs/current-native-namespace-tail121/probe.c')
output = pathlib.Path('/workspace/.proofs/captured-native-namespace-tail121')
output.mkdir()
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
captures = []
for version in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0', 'jim']:
    directory = output / version
    directory.mkdir()
    executable = directory / 'probe'
    environment = os.environ.copy()
    if version == 'jim':
        source = pathlib.Path('/workspace/.proofs/native-providers/jimtcl')
        header, library = source / 'jim.h', source / 'libjim.a'
        flags = ['-DUSE_JIM=1', '-I' + str(source)]
        libraries = ['-lm', '-lssl', '-lcrypto', '-lz', '-ldl']
        identity = subprocess.check_output(['git', 'describe', '--tags', '--always'], cwd=source).decode().strip()
    else:
        source = repo / 'tmp' / ('tcl' + version)
        header = source / 'generic' / 'tcl.h'
        library = source / 'unix' / ('libtcl' + '.'.join(version.split('.')[:2]) + '.a')
        flags = ['-I' + str(header.parent), '-I' + str(source / 'unix')]
        libraries = ['-lm', '-ldl', '-lpthread', '-lz']
        environment['TCL_LIBRARY'] = str(source / 'library')
        identity = version
    command = ['cc', '-std=c99', *flags, str(probe), str(library), *libraries, '-o', str(executable)]
    compile_result = subprocess.run(command, capture_output=True, timeout=60)
    (directory / 'compile.stdout').write_bytes(compile_result.stdout)
    (directory / 'compile.stderr').write_bytes(compile_result.stderr)
    if compile_result.returncode:
        raise RuntimeError('compile failed ' + version)
    result = subprocess.run([str(executable)], env=environment, capture_output=True, timeout=60)
    (directory / 'stdout.tsv').write_bytes(result.stdout)
    (directory / 'stderr').write_bytes(result.stderr)
    receipt = {'provider': identity, 'probe_sha256': sha(probe), 'header_sha256': sha(header),
               'library_sha256': sha(library), 'compile_command': command,
               'compile_exit': compile_result.returncode, 'executable_sha256': sha(executable),
               'process_exit': result.returncode, 'stdout_sha256': sha(directory / 'stdout.tsv'),
               'stderr_sha256': sha(directory / 'stderr'), 'rows': result.stdout.decode().splitlines()}
    (directory / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    captures.append(receipt)
    (output / 'receipt.json').write_text(json.dumps(captures, indent=2) + '\n')
    print(identity, 'process', result.returncode, 'rows', len(receipt['rows']), flush=True)
    if result.returncode or result.stderr:
        raise RuntimeError('harness failed ' + version)
