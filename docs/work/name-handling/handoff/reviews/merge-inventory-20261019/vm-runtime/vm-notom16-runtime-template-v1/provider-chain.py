from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,sys
records={}; pending=[]; cache={}
for name,value in os.environ.items():
    if name.startswith('TCL_LSP_') and ('TCLSH' in name or 'JIMSH' in name or 'COMPILER' in name):
        if Path(value).is_file(): pending.append((name,Path(value)))
for tool in ['cargo','rustc','rustfmt','clippy-driver']:
    proxy=shutil.which(tool)
    if proxy: pending.append((tool+'-proxy',Path(proxy)))
    root=Path(os.environ.get('RUSTUP_HOME','/workspace/.rustup'))/'toolchains'
    for candidate in root.glob(os.environ.get('RUSTUP_TOOLCHAIN','1.99.1')+'*/bin/'+tool):
        pending.append((tool+'-actual',candidate))
for tool in ['cc','ar','as','ld']:
    path=shutil.which(tool)
    assert path, 'missing actual C tool '+tool
    pending.append((tool+'-actual',Path(path)))
for name in ['cc1','collect2']:
    actual=subprocess.check_output(['/usr/bin/cc','-print-prog-name='+name],text=True).strip()
    path=Path(actual) if '/' in actual else Path(shutil.which(actual) or '')
    assert path.is_file(), 'missing actual C worker '+name
    pending.append((name+'-actual',path))
seen=set()
while pending:
    label,path=pending.pop()
    real=path.resolve()
    if not real.is_file(): continue
    key=str(real)
    if key not in cache:
        with real.open('rb') as stream: digest=hashlib.file_digest(stream,'sha256').hexdigest()
        cache[key]={'realpath':key,'sha256':digest,'size':real.stat().st_size}
    record={'path':str(path),**cache[key]}
    records[label+':'+str(path)]=record
    if key in seen: continue
    seen.add(key)
    with real.open('rb') as stream: header=stream.read(8192)
    if header.startswith(b'#!'):
        # Pinned tclsh wrappers exec a quoted absolute original worker.
        for target in re.findall(rb'^exec\s+"(/[^"\n]+)"',header,re.M):
            pending.append((label+'-worker',Path(os.fsdecode(target))))
    elif header.startswith(b'\x7fELF'):
        linked=subprocess.run(['ldd',str(real)],capture_output=True,text=True,check=False)
        for target in re.findall(r'(?:=>\s+|^\s*)(/[^\s()]+)',linked.stdout,re.M):
            dependency=Path(target)
            if dependency.is_file(): pending.append((label+'-shared',dependency))
Path(sys.argv[1]).write_text(json.dumps(records,sort_keys=True,indent=2)+'\n')
