import pathlib, subprocess, hashlib, json, os

repo = pathlib.Path('/workspace/tcl-lsp')
probe = pathlib.Path('/workspace/.proofs/native-brace-expression129-v4/probe.c')
output = pathlib.Path('/workspace/.proofs/captured-native-brace-expression129-v4')
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
        flags = ['-DJIM_PROBE=1', '-I' + str(source)]
        libraries = ['-lm', '-lssl', '-lcrypto', '-lz', '-ldl']
        identity = 'jim-current-upstream'
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
    result = subprocess.run([str(executable), '/workspace/.proofs/native-brace-expression129-v4/inputs'], env=environment, capture_output=True, timeout=60)
    (directory / 'stdout.tsv').write_bytes(result.stdout)
    (directory / 'stderr').write_bytes(result.stderr)
    receipt = {'provider': identity, 'probe_sha256': sha(probe), 'runner_sha256': sha(pathlib.Path(__file__)), 'case_header_sha256': sha(probe.parent / 'cases.h'), 'inputs_manifest_sha256': sha(probe.parent / 'inputs.json'), 'input_file_sha256': {str(path.relative_to(probe.parent)): sha(path) for path in sorted((probe.parent / 'inputs').rglob('*')) if path.is_file()}, 'header_sha256': sha(header), 'makefile_sha256': sha(source / ('Makefile' if version == 'jim' else 'unix/Makefile')),
               'source_owner_sha256': sha(source / ('jim.c' if version == 'jim' else 'generic/tclInterp.c')),
               'additional_source_sha256': {name: sha(source / 'generic' / name) for name in ['tclBasic.c', 'tclListObj.c', 'tclParse.c', 'tclCmdIL.c', 'tclCmdMZ.c', 'tclEncoding.c', 'tclIO.c'] if (source / 'generic' / name).exists()}, 'library_sha256': sha(library), 'compile_command': command,
               'compile_exit': compile_result.returncode, 'executable_sha256': sha(executable),
               'process_exit': result.returncode, 'stdout_sha256': sha(directory / 'stdout.tsv'),
               'stderr_sha256': sha(directory / 'stderr'), 'rows': result.stdout.decode('ascii').splitlines(), 'reported_version': bytes.fromhex(next(row for row in result.stdout.decode('ascii').splitlines() if row.startswith('VALUE|startup|provider|counted-native|version|result|')).split('|')[-1]).decode('ascii')}
    (directory / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    captures.append(receipt)
    (output / 'receipt.json').write_text(json.dumps(captures, indent=2) + '\n')
    print(identity, 'process', result.returncode, 'rows', len(receipt['rows']), flush=True)
    print(result.stdout.decode(), flush=True)
    if result.returncode or result.stderr:
        raise RuntimeError('harness failed ' + version)
