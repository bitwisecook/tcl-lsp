import pathlib, subprocess, hashlib, json, os, re, shutil, time

repo = pathlib.Path('/workspace/tcl-lsp')
probe = pathlib.Path('/workspace/.proofs/current-native-vwait-forms137/probe.c')
output = pathlib.Path('/workspace/.proofs/captured-native-vwait-forms137')
output.mkdir()
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
captures = []
compiler=pathlib.Path(shutil.which('cc')).resolve()
compiler_identity=subprocess.check_output([str(compiler),'--version']).decode()
cases=re.findall(r'\{"([a-z-]+)",',probe.read_text())
assert len(cases)==15
(output/'probe.c').write_bytes(probe.read_bytes())
(output/'queue.json').write_bytes(probe.with_name('queue.json').read_bytes())
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
    attempts=[]
    for case in cases:
        start=time.monotonic()
        try:
            result=subprocess.run([str(executable),case],env=environment,capture_output=True,timeout=3)
            stdout,stderr,exitcode,timedout=result.stdout,result.stderr,result.returncode,False
        except subprocess.TimeoutExpired as expired:
            stdout,stderr,exitcode,timedout=expired.stdout or b'',expired.stderr or b'',None,True
        stem=directory/case
        stem.with_suffix('.stdout.tsv').write_bytes(stdout)
        stem.with_suffix('.stderr').write_bytes(stderr)
        attempt={'case':case,'command':[str(executable),case],'external_timeout':timedout,'timeout_seconds':3,'process_exit':exitcode,'seconds':time.monotonic()-start,'stdout_sha256':sha(stem.with_suffix('.stdout.tsv')),'stderr_sha256':sha(stem.with_suffix('.stderr')),'rows':stdout.decode().splitlines()}
        stem.with_suffix('.json').write_text(json.dumps(attempt,indent=2)+'\n');attempts.append(attempt)
        print(identity,case,'exit',exitcode,'timeout',timedout,'rows',len(attempt['rows']),flush=True)
    receipt={'sdk_provider_label':identity,'actual_provider_rows':[r for a in attempts for r in a['rows'] if r.startswith('V|')],'probe_sha256':sha(probe),'header_sha256':sha(header),'library_sha256':sha(library),'compiler_path':str(compiler),'compiler_sha256':sha(compiler),'compiler_version':compiler_identity,'compile_command':command,'compile_exit':compile_result.returncode,'compile_stdout_sha256':sha(directory/'compile.stdout'),'compile_stderr_sha256':sha(directory/'compile.stderr'),'executable_sha256':sha(executable),'attempts':attempts,'claim_boundary':'Only completed guest rows answer cases. External timeouts are capture limitations and retain original partial streams.'}
    (directory/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');captures.append(receipt)
    (output/'receipt.json').write_text(json.dumps(captures,indent=2)+'\n')
