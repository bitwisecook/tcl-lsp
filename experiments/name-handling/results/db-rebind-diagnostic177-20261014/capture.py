from pathlib import Path
import argparse,datetime,difflib,gzip,hashlib,json,os,re,subprocess,time,threading
ap=argparse.ArgumentParser();ap.add_argument('--package',required=True);ap.add_argument('--plan',required=True);ap.add_argument('--out',required=True);ap.add_argument('--binary');ap.add_argument('--label',required=True);ap.add_argument('--expected-source');ap.add_argument('--timeout',type=int,default=180);ap.add_argument('--jobs',type=int,default=2);a=ap.parse_args()
root=Path('/workspace/tcl-lsp');out=Path(a.out);out.mkdir(parents=True,exist_ok=False)
plan=json.loads(Path(a.plan).read_text());filters=plan.get('filters',[]);exact=plan.get('exact',[])
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def inventory():
    names=set(subprocess.check_output(['rg','--files','--hidden',*[n for n in ['rust','runtime/rust','data'] if (root/n).exists()]],cwd=root,text=True).splitlines())
    names.update(['Cargo.toml','Cargo.lock','rust-toolchain.toml','experiments/name-handling/tcltest-shared-frame/probe.tcl'])
    for base in [root/'.cargo',root/'experiments/name-handling/patterns/composed-callback-prefix/fixtures']:
        if base.exists():
            names.update(str(p.relative_to(root)) for p in base.rglob('*') if p.is_file())
    return {n:sha(root/n) for n in sorted(names) if (root/n).is_file()}
def snap(name,value):
    with gzip.open(out/name,'wt') as f:json.dump(value,f,sort_keys=True)
def run(command,name):
    timeout = plan.get('test_timeouts', {}).get(command[1], a.timeout) if name.startswith('test-') else None
    t=time.monotonic()
    if name.startswith('test-'):
        name += '.gz'
        process=subprocess.Popen(command,cwd=root/'rust',stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        def preserve_output():
            with gzip.open(out/name,'wb',compresslevel=6) as f:
                while chunk:=process.stdout.read(65536):f.write(chunk)
        writer=threading.Thread(target=preserve_output);writer.start()
        timed_out=False
        try:code=process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out=True;process.kill();code=process.wait()
        writer.join()
    else:
        with (out/name).open('wb') as f:
            process=subprocess.Popen(command,cwd=root/'rust',stdout=f,stderr=subprocess.STDOUT)
            timed_out=False
            try:code=process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out=True;process.kill();code=process.wait()
    return {'command':command,'exit_code':code,'seconds':time.monotonic()-t,'log':name,'timed_out':timed_out}
source=inventory();snap('source-before.json.gz',source)
if a.expected_source:
    with gzip.open(a.expected_source,'rt') as f:expected=json.load(f)
    mismatches={n:{'expected':h,'current':sha(root/n) if (root/n).is_file() else None} for n,h in expected.items() if not (root/n).is_file() or sha(root/n)!=h}
    (out/'expected-source-comparison.json').write_text(json.dumps({'expected_source':a.expected_source,'expected_paths':len(expected),'mismatches':mismatches},indent=2)+'\n')
    if mismatches:raise SystemExit('Current source mismatches binary production capture; no tests run.')
start={'started_utc':now(),'label':a.label,'package':a.package,'target':os.environ.get('CARGO_TARGET_DIR'),'source_paths':len(source),'plan':plan,'rustc':subprocess.check_output(['rustc','-Vv'],text=True),'cargo':subprocess.check_output(['cargo','-V'],text=True),'runner_sha256':sha(Path(__file__)), 'source_channel':('temporary cfg(test) stage diagnostics' if plan.get('temporary_diagnostics') else 'production frozen; no temporary diagnostics')}
(out/'start.json').write_text(json.dumps(start,indent=2)+'\n')
result={'started_utc':start['started_utc'],'build':None,'run':None}
binary=Path(a.binary) if a.binary else None
if binary is None:
    cmd=['cargo','test','--offline','-j',str(a.jobs),'-p',a.package,'--lib','--no-run','--message-format=json']
    result['build']=run(cmd,'build.log')
    if result['build']['exit_code']==0:
        for line in (out/'build.log').read_text().splitlines():
            try:row=json.loads(line)
            except json.JSONDecodeError:continue
            if row.get('reason')=='compiler-artifact' and row.get('profile',{}).get('test') and row.get('target',{}).get('name')==a.package.replace('-','_') and row.get('executable'):
                binary=Path(row['executable'])
if binary is not None:
    result['binary']={'path':str(binary),'sha256':sha(binary),'bytes':binary.stat().st_size}
    listing=subprocess.check_output([str(binary),'--list'],text=True)
    (out/'test-list.txt').write_text(listing)
    names=[line.removesuffix(': test') for line in listing.splitlines() if line.endswith(': test')]
    selected=[n for n in names if n in exact or any(f in n for f in filters)]
    priority=plan.get('priority_filters',[])
    selected.sort(key=lambda n: next((i for i,f in enumerate(priority) if f in n),len(priority)))
    missing=[n for n in exact if n not in names]
    nofilters=[f for f in filters if not any(f in n for n in names)]
    result['selected_tests']=selected;result['missing_exact']=missing;result['empty_filters']=nofilters
    if selected and not missing and not nofilters:
        result['runs']=[]
        for index,name in enumerate(selected):
            print('Starting exact test: '+name,flush=True)
            row=run([str(binary),name,'--exact','--nocapture','--test-threads=1'],f'test-{index+1:02d}.log')
            row['test']=name; row['summaries']=[]; row['running_counts']=[]
            with gzip.open(out/row['log'],'rt',errors='replace') as stream:
                previous=''
                for chunk in iter(lambda:stream.read(65536),''):
                    window=previous+chunk
                    boundary=len(previous)
                    row['summaries'] += [match.groups() for match in re.finditer(r'test result: (\w+)\. (\d+) passed; (\d+) failed;',window) if match.end()>boundary]
                    row['running_counts'] += [match.group(1) for match in re.finditer(r'running (\d+) tests?',window) if match.end()>boundary]
                    previous=chunk[-256:]
            result['runs'].append(row)
            print(json.dumps(row),flush=True)
        result['summaries']=[summary for row in result['runs'] for summary in row['summaries']]
    else:result['selection_error']='one or more required selectors absent; no tests executed'
end=inventory();snap('source-after.json.gz',end)
result['finished_utc']=now();result['changed_sources']={n:{'before':source.get(n),'after':end.get(n)} for n in set(source)|set(end) if source.get(n)!=end.get(n)}
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result),flush=True)
