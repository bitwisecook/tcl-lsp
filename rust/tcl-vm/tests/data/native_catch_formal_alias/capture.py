import hashlib,json,os,pathlib,subprocess
root=pathlib.Path(__file__).resolve().parent
repo=pathlib.Path('/workspace/tcl-lsp')
source=root/'cases.tcl'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
records=[]
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim']:
    base=repo/'tmp'/('tcl'+version) if version!='jim' else pathlib.Path('/workspace/.proofs/native-providers/jimtcl')
    executable=base/'unix/tclsh' if version!='jim' else base/'jimsh'
    header=base/'generic/tcl.h' if version!='jim' else base/'jim.h'
    makefile=base/'unix/Makefile' if version!='jim' else base/'Makefile'
    owner=base/'generic/tclParse.c' if version!='jim' else base/'jim.c'
    env=os.environ.copy()
    if version!='jim':env['TCL_LIBRARY']=str(base/'library')
    out=root/version;out.mkdir(exist_ok=False)
    process=subprocess.run([str(executable)],input=source.read_bytes(),capture_output=True,env=env,timeout=30,check=False)
    (out/'stdout').write_bytes(process.stdout);(out/'stderr').write_bytes(process.stderr)
    rows=process.stdout.decode('ascii').splitlines()
    row={'provider':version,'input_channel':'ASCII file bytes passed to shell stdin','source_sha256':sha(source),'executable_path':str(executable),'executable_sha256':sha(executable),'header_sha256':sha(header),'source_owner_sha256':sha(owner),'makefile_sha256':sha(makefile),'command':[str(executable)],'TCL_LIBRARY':env.get('TCL_LIBRARY') if version!='jim' else None,'process_exit':process.returncode,'stdout_sha256':sha(out/'stdout'),'stderr_sha256':sha(out/'stderr'),'reported_version':rows[0].split('|',1)[1] if rows and rows[0].startswith('VERSION|') else None,'rows':rows}
    (out/'receipt.json').write_text(json.dumps(row,indent=2)+'\n');records.append(row)
    (root/'receipt.json').write_text(json.dumps(records,indent=2)+'\n')
    print(version,process.returncode,rows,flush=True)
    if process.returncode or process.stderr or len(rows)!=4:raise RuntimeError('harness failed '+version)
