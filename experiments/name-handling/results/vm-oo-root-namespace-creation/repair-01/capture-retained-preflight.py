import hashlib, json, os, pathlib, shlex, subprocess
repo = pathlib.Path('/workspace/tcl-lsp')
out = pathlib.Path(__file__).resolve().parent
source = repo / 'tmp/tcl9.1.0'
probe = out / 'probe.c'
executable = out / 'probe-c91'
retained = json.loads((repo / 'rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.1.0/receipt.json').read_text())
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
makefile = source / 'unix/Makefile'
library = source / 'unix/libtcl9.1.a'
paths = [probe, pathlib.Path(__file__).resolve(), makefile, library, source / 'generic/tcl.h']
paths += [source / name for name in retained['source_sha256']]
paths += [source / name for name in retained['private_headers_sha256']]
before = {str(path): sha(path) for path in paths}
assert sha(library) == retained['library_sha256']
assert sha(source / 'generic/tcl.h') == retained['header_sha256']
assert all(sha(source / name) == digest for name, digest in retained['source_sha256'].items())
definition = next(line.partition('=')[2].strip() for line in makefile.read_text().splitlines() if line.split('=', 1)[0].strip() == 'AC_FLAGS')
command = ['cc', '-std=c99', '-D_GNU_SOURCE', *shlex.split(definition), '-I'+str(source / 'generic'), '-I'+str(source / 'unix'), str(probe), str(library), '-lm', '-ldl', '-lpthread', '-lz', '-o', str(executable)]
environment = os.environ.copy()
environment['TCL_LIBRARY'] = str(source / 'library')
compile_result = subprocess.run(command, capture_output=True, timeout=60)
(out / 'c91-compile.stdout').write_bytes(compile_result.stdout)
(out / 'c91-compile.stderr').write_bytes(compile_result.stderr)
receipt = {'provider': 'C Tcl 9.1.0', 'compile_command': command, 'compile_exit': compile_result.returncode, 'environment': {'TCL_LIBRARY': environment['TCL_LIBRARY']}, 'source_before_sha256': before, 'retained_receipt_sha256': sha(repo / 'rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3/9.1.0/receipt.json'), 'process_exit': None}
if compile_result.returncode == 0:
    result = subprocess.run([str(executable)], env=environment, capture_output=True, timeout=60)
    (out / 'c91-stdout.tsv').write_bytes(result.stdout)
    (out / 'c91-stderr').write_bytes(result.stderr)
    receipt.update({'process_exit': result.returncode, 'executable_sha256': sha(executable), 'stdout_sha256': sha(out / 'c91-stdout.tsv'), 'stderr_sha256': sha(out / 'c91-stderr'), 'rows': result.stdout.decode('ascii').splitlines()})
receipt['source_after_sha256'] = {str(path): sha(path) for path in paths}
receipt['source_unchanged'] = receipt['source_before_sha256'] == receipt['source_after_sha256']
receipt['compile_stdout_sha256'] = sha(out / 'c91-compile.stdout')
receipt['compile_stderr_sha256'] = sha(out / 'c91-compile.stderr')
(out / 'c91-receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')
print(json.dumps({key: receipt[key] for key in ['compile_exit','process_exit','source_unchanged']}, indent=2))
if receipt['process_exit'] != 0 or not receipt['source_unchanged']:
    raise SystemExit(1)
