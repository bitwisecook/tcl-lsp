from pathlib import Path
import hashlib,json,os,shutil,subprocess,time

packet=Path('/workspace/.proofs/current-native-property-counted144')
output=Path('/workspace/.proofs/captured-native-property-counted145')
queue=json.loads((packet/'queue.json').read_text())
providers=json.loads(Path('/workspace/.proofs/native-info-inventory193/queue.json').read_text())['providers']
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
assert sha(packet/'probe.c')==queue['source_sha256']
compiler=Path(shutil.which('cc')).resolve()
compiler_version=subprocess.check_output([str(compiler),'--version']).decode()
output.mkdir()
records=[]
for provider in providers:
    pins=dict(provider['required_sha256'])
    root=Path(provider['library']).parent.parent
    source=Path('/workspace/.proofs/native-providers/jimtcl/jim.c') if provider['id']=='jim' else root/'generic/tclOOProp.c' if (root/'generic/tclOOProp.c').exists() else root/'generic/tclBasic.c'
    pins[str(source)]=sha(source)
    for path,digest in pins.items():assert sha(path)==digest,path
    directory=output/provider['id'];directory.mkdir();executable=directory/'probe'
    command=[str(compiler),'-std=c99',*provider['flags'],str(packet/'probe.c'),provider['library'],*provider['link_flags'],'-o',str(executable)]
    build=subprocess.run(command,capture_output=True,timeout=60)
    (directory/'compile.stdout').write_bytes(build.stdout);(directory/'compile.stderr').write_bytes(build.stderr)
    record=dict(provider=provider['id'],compiler_path=str(compiler),compiler_sha256=sha(compiler),compiler_version=compiler_version,compile_command=command,compile_exit=build.returncode,required_sha256=pins,environment=provider['environment'],probe_sha256=sha(packet/'probe.c'),queue_sha256=sha(packet/'queue.json'),runner_sha256=sha(__file__),compile_stdout_sha256=sha(directory/'compile.stdout'),compile_stderr_sha256=sha(directory/'compile.stderr'))
    if build.returncode==0:
        start=time.monotonic()
        run=subprocess.run([str(executable)],env={**os.environ,**provider['environment']},capture_output=True,timeout=60)
        (directory/'stdout.tsv').write_bytes(run.stdout);(directory/'stderr').write_bytes(run.stderr)
        rows=run.stdout.decode('ascii').splitlines();versions=[row for row in rows if row.startswith('VERSION|0|')]
        assert len(versions)==1,provider['id']
        actual=bytes.fromhex(versions[0].split('|')[-1]).decode('ascii')
        assert actual.startswith('0.84-9-g5bac7c9') if provider['id']=='jim' else actual==provider['id'],actual
        record.update(executable_sha256=sha(executable),command=[str(executable)],process_exit=run.returncode,seconds=time.monotonic()-start,stdout_sha256=sha(directory/'stdout.tsv'),stderr_sha256=sha(directory/'stderr'),rows=rows,reported_version=actual)
    (directory/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');records.append(record)
    (output/'receipt.json').write_text(json.dumps(records,indent=2)+'\n')
    print(provider['id'],'compile',build.returncode,'process',record.get('process_exit'),'rows',len(record.get('rows',[])),flush=True)
    assert build.returncode==0 and record['process_exit']==0 and not run.stderr
