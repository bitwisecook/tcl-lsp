import hashlib
import json
import os
import pathlib
import subprocess

packet = pathlib.Path('/workspace/.proofs/native-child-command-lifetime078-v2')
output = pathlib.Path('/workspace/.proofs/captured-native-child-command-lifetime078-v2')
repo = pathlib.Path('/workspace/tcl-lsp')
output.mkdir()
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
all_receipts = []
for version in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0', 'jim']:
    directory = output / version
    directory.mkdir()
    executable = directory / 'probe'
    environment = os.environ.copy()
    if version == 'jim':
        source = pathlib.Path('/workspace/.proofs/native-providers/jimtcl')
        header, library, makefile, owner = source / 'jim.h', source / 'libjim.a', source / 'Makefile', source / 'jim.c'
        flags = ['-DJIM_PROBE=1', '-I' + str(source)]
        libraries = ['-lm', '-lssl', '-lcrypto', '-lz', '-ldl']
    else:
        source = repo / 'tmp' / ('tcl' + version)
        header = source / 'generic/tcl.h'
        library = source / 'unix' / ('libtcl' + '.'.join(version.split('.')[:2]) + '.a')
        makefile, owner = source / 'unix/Makefile', source / 'generic/tclInterp.c'
        flags = ['-I' + str(header.parent), '-I' + str(source / 'unix')]
        libraries = ['-lm', '-ldl', '-lpthread', '-lz']
        environment['TCL_LIBRARY'] = str(source / 'library')
    command = ['cc', '-std=c99', *flags, str(packet / 'probe.c'), str(library), *libraries, '-o', str(executable)]
    compiled = subprocess.run(command, capture_output=True, timeout=60)
    (directory / 'compile.stdout').write_bytes(compiled.stdout)
    (directory / 'compile.stderr').write_bytes(compiled.stderr)
    receipt = dict(provider=version, probe_sha256=sha(packet / 'probe.c'), runner_sha256=sha(pathlib.Path(__file__)),
                   cases_sha256=sha(packet / 'cases.h'), inputs_sha256=sha(packet / 'inputs.json'),
                   input_file_sha256={str(p.relative_to(packet)):sha(p) for p in sorted((packet / 'inputs').glob('*.tcl'))},
                   header_sha256=sha(header), library_sha256=sha(library), makefile_sha256=sha(makefile), source_owner_sha256=sha(owner),
                   compile_command=command, compile_exit=compiled.returncode,
                   compile_stdout_sha256=sha(directory / 'compile.stdout'), compile_stderr_sha256=sha(directory / 'compile.stderr'))
    (directory / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    if compiled.returncode:
        raise RuntimeError('compile failed ' + version)
    result = subprocess.run([str(executable)], env=environment, capture_output=True, timeout=30)
    (directory / 'stdout.tsv').write_bytes(result.stdout)
    (directory / 'stderr').write_bytes(result.stderr)
    rows = result.stdout.decode('ascii').splitlines()
    version_rows = [r for r in rows if r.startswith('VALUE|startup|version|result|')]
    receipt.update(executable_sha256=sha(executable), process_exit=result.returncode,
                   stdout_sha256=sha(directory / 'stdout.tsv'), stderr_sha256=sha(directory / 'stderr'), rows=rows,
                   reported_version=bytes.fromhex(version_rows[0].split('|')[-1]).decode('ascii') if version_rows else None)
    (directory / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    all_receipts.append(receipt)
    (output / 'receipt.json').write_text(json.dumps(all_receipts, indent=2) + '\n')
    print(version, 'compile', compiled.returncode, 'process', result.returncode, 'rows', len(rows), flush=True)
    if result.returncode or result.stderr:
        raise RuntimeError('probe process failed ' + version)
