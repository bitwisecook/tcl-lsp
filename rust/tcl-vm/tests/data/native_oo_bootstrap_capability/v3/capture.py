import hashlib
import json
import os
import pathlib
import shlex
import subprocess

repo = pathlib.Path('/workspace/tcl-lsp')
probe = pathlib.Path('/workspace/.proofs/native-oo-bootstrap-capability074-v3/probe.c')
output = pathlib.Path('/workspace/.proofs/captured-native-oo-bootstrap-capability074-v3')
output.mkdir()

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

captures = []
failed = False
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
        owner_names = ['jim.c']
        private_names = []
        makefile = source / 'Makefile'
    else:
        source = repo / 'tmp' / ('tcl' + version)
        header = source / 'generic' / 'tcl.h'
        library = source / 'unix' / ('libtcl' + '.'.join(version.split('.')[:2]) + '.a')
        makefile = source / 'unix' / 'Makefile'
        definition = next(line.partition('=')[2].strip()
                          for line in makefile.read_text().splitlines()
                          if line.split('=', 1)[0].strip() == 'AC_FLAGS')
        flags = ['-D_GNU_SOURCE', *shlex.split(definition), '-I' + str(header.parent), '-I' + str(source / 'unix')]
        libraries = ['-lm', '-ldl', '-lpthread', '-lz']
        environment['TCL_LIBRARY'] = str(source / 'library')
        identity = version
        owner_names = ['generic/tclOO.c', 'generic/tclOOBasic.c', 'generic/tclOODefineCmds.c', 'generic/tclOOCall.c']
        private_names = ['generic/tclInt.h', 'generic/tclOOInt.h', 'generic/tclOOIntDecls.h']
    command = ['cc', '-std=c99', *flags, str(probe), str(library), *libraries, '-o', str(executable)]
    compile_result = subprocess.run(command, capture_output=True, timeout=60)
    (directory / 'compile.stdout').write_bytes(compile_result.stdout)
    (directory / 'compile.stderr').write_bytes(compile_result.stderr)
    receipt = {
        'provider': identity,
        'probe_sha256': sha(probe),
        'runner_sha256': sha(pathlib.Path(__file__)),
        'header_sha256': sha(header),
        'makefile_sha256': sha(makefile),
        'source_sha256': {name: sha(source / name) for name in owner_names if (source / name).exists()},
        'private_headers_sha256': {name: sha(source / name) for name in private_names if (source / name).exists()},
        'library_sha256': sha(library),
        'compile_command': command,
        'compile_exit': compile_result.returncode,
        'compile_stdout_sha256': sha(directory / 'compile.stdout'),
        'compile_stderr_sha256': sha(directory / 'compile.stderr'),
        'environment': {'TCL_LIBRARY': environment.get('TCL_LIBRARY')},
        'process_exit': None,
        'rows': [],
    }
    if compile_result.returncode == 0:
        result = subprocess.run([str(executable)], env=environment, capture_output=True, timeout=60)
        (directory / 'stdout.tsv').write_bytes(result.stdout)
        (directory / 'stderr').write_bytes(result.stderr)
        rows = result.stdout.decode('ascii').splitlines()
        version_rows = [row for row in rows if row.startswith('VERSION|0|')]
        receipt.update({
            'executable_sha256': sha(executable),
            'process_exit': result.returncode,
            'stdout_sha256': sha(directory / 'stdout.tsv'),
            'stderr_sha256': sha(directory / 'stderr'),
            'rows': rows,
            'reported_version': bytes.fromhex(version_rows[0].split('|')[2]).decode('ascii') if len(version_rows) == 1 else None,
        })
        failed |= result.returncode != 0 or bool(result.stderr)
    else:
        failed = True
    (directory / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    captures.append(receipt)
    (output / 'receipt.json').write_text(json.dumps(captures, indent=2) + '\n')
    print(identity, 'compile', receipt['compile_exit'], 'process', receipt['process_exit'], 'rows', len(receipt['rows']), flush=True)
    for row in receipt['rows']:
        print(row, flush=True)
if failed:
    raise RuntimeError('one or more original bootstrap probes failed; all attempt receipts retained')
