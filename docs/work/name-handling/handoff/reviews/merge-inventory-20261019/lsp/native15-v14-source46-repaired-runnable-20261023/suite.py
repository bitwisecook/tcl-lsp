#!/usr/bin/env python3
"""Exact LSP native 15; immutable preparation, root GO required."""
from pathlib import Path
import datetime, fcntl, hashlib, importlib.util, json, os, re, shutil, subprocess, sys

ROOT=Path('/workspace/naming-migration-active/tcl-lsp')
BASE=Path('/tmp/lsp-native-protocol-v14-source46-repaired-direct15')
PLAN=BASE/'plan.json'
GO=BASE/'root-go.json'
MANIFEST=BASE/'runner-input-manifest.json'
SOURCE46=Path('/tmp/name-handling-eight-package-check-v9-source46/source46-manifest.json')
CAPTURE=ROOT/'docs/work/name-handling/validation-tools/capture-inputs.py'
LOCK=Path('/tmp/tcl-lsp-naming-build.lock')
CARGO=Path('/workspace/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/cargo')
RUNS=Path('/tmp/lsp-native-protocol-v14-source46-repaired-direct15-runs')

def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()
def utc(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
def save(path,value): Path(path).write_text(json.dumps(value,sort_keys=True,indent=2)+'\n')
def git(*args): return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
def require(condition,message):
    if not condition: raise RuntimeError('STOP: '+message)
def environment_override_keys(plan, environment):
    exact=set(plan['refused_environment_exact'])
    return sorted(key for key in environment if key in exact or any(key.startswith(pattern['prefix']) and key.endswith(pattern['suffix']) for pattern in plan['refused_environment_patterns']))

def source_view(go,go_sha,plan):
    global last_head, head_samples
    require(sha(GO)==go_sha,'root GO changed')
    require(not environment_override_keys(plan,os.environ),'unsupported build environment override')
    require(str(CARGO)==plan['cargo_path'] and sha(CARGO)==plan['cargo_sha256'],'pinned Cargo path/hash changed')
    require(sha(plan['rustc_path'])==plan['rustc_sha256'],'pinned rustc changed')
    require(all(os.environ.get(key)==value for key,value in plan['required_environment'].items()),'required build environment mismatch')
    require(os.environ.get('PATH','').startswith(plan['path_prefix']+':'),'required tool PATH prefix absent')
    require(go.get('plan_sha256')==sha(PLAN) and go.get('runner_input_manifest_sha256')==sha(MANIFEST),'GO plan/runner pin mismatch')
    require(go.get('source46_manifest_sha256')==sha(SOURCE46)==plan['lifecycle']['source46_manifest_sha256'],'source46 manifest pin mismatch')
    approved=go.get('approved_integrated_head')
    require(isinstance(approved,str) and re.fullmatch(r'[0-9a-f]{40}',approved),'missing exact approved HEAD')
    require(approved==plan['approved_product_head'],'GO differs from actual integrated source46 product')
    approved_tree=go.get('approved_integrated_tree')
    require(isinstance(approved_tree,str) and re.fullmatch(r'[0-9a-f]{40}',approved_tree),'missing exact approved tree')
    require(git('rev-parse',approved+'^{tree}')==approved_tree==plan['approved_product_tree'],'approved tree mismatch')
    require(subprocess.run(['git','merge-base','--is-ancestor','3b1a750be337',approved],cwd=ROOT,check=False).returncode==0,'approved HEAD not after baseline')
    current=git('rev-parse','HEAD')
    require(subprocess.run(['git','merge-base','--is-ancestor',approved,current],cwd=ROOT,check=False).returncode==0,'current HEAD not descendant of approved HEAD')
    commits=git('rev-list',approved+'..'+current).splitlines()
    for commit in commits:
        changed=git('diff-tree','--no-commit-id','--name-only','-r','-m',commit).splitlines()
        require(all(p.startswith('docs/work/name-handling/') for p in changed),'non-doc descendant commit: '+commit)
    tracked=subprocess.check_output(['git','diff','--name-only',approved,'--','.',':(exclude)docs/work/name-handling'],cwd=ROOT,text=True).splitlines()
    untracked=subprocess.check_output(['git','ls-files','--others','--exclude-standard','-z'],cwd=ROOT).split(b'\0')
    require(not tracked and all(not p or p.decode(errors='surrogateescape').startswith('docs/work/name-handling/') for p in untracked),'unapproved worktree product/untracked delta')
    for path,expected in json.loads(MANIFEST.read_text())['inputs'].items(): require(sha(path)==expected,'runner input changed: '+path)
    for item in plan['source_pins']: require(sha(ROOT/item['path'])==item['sha256'],'source pin changed: '+item['path'])
    source_rows=json.loads(SOURCE46.read_text())['paths']
    require(len(source_rows)==46 and len({row['path'] for row in source_rows})==46,'source46 inventory changed')
    for row in source_rows:
        path=row['path']; expected=row['after_sha256']
        require(sha(ROOT/path)==expected,'source46 final hash changed: '+path)
        committed=subprocess.run(['git','show',approved+':'+path],cwd=ROOT,capture_output=True)
        require(committed.returncode==0 and hashlib.sha256(committed.stdout).hexdigest()==expected,'approved product source46 blob mismatch: '+path)
    require(subprocess.run(['git','diff','--quiet',approved,'--','.',':(exclude)docs/work/name-handling'],cwd=ROOT).returncode==0,'worktree product differs from approved HEAD/tree')
    for key in ('prior_report14_context_evidence','post46_eight_package_typecheck_evidence'):
        evidence=go.get(key)
        require(isinstance(evidence,dict) and evidence.get('path') and evidence.get('sha256') and sha(evidence['path'])==evidence['sha256'],'gate evidence missing/changed: '+key)
    typing=json.loads(Path(go['post46_eight_package_typecheck_evidence']['path']).read_text())
    require(typing.get('status')=='PASS' and typing.get('overall_exit')==0 and typing.get('cargo_exit_code')==0,'eight-package check not PASS0')
    require(all(typing.get(key) is True for key in ('guard_valid','warning_free_json_stderr_stdout','source_stable','providers_stable','root_go_stable','head_docs_descendant_valid')),'eight-package check guard/warning seal incomplete')
    require(typing.get('plan_sha256')==plan['lifecycle']['post46_eight_package_typecheck_plan_sha256'],'wrong eight-package check plan')
    require(typing.get('approved_product_head')==approved and typing.get('approved_product_tree')==approved_tree,'eight-package product HEAD/tree mismatch')
    require(typing.get('selected_packages')==plan['required_typecheck_packages'],'eight-package check package inventory mismatch')
    if last_head is not None:
        require(subprocess.run(['git','merge-base','--is-ancestor',last_head,current],cwd=ROOT).returncode==0,'actual HEAD moved outside monotonic docs-only history')
    head_samples.append({'utc':utc(),'actual_head':current})
    last_head=current
    return current

def disk(paths):
    result={}
    for raw in paths:
        path=Path(raw)
        while not path.exists(): path=path.parent
        result[str(raw)]=shutil.disk_usage(path).free
    require(min(result.values())>=2300000000,'actual mount below 2.3GB floor: '+str(result))
    return result

def snapshot(prefix):
    subprocess.run([sys.executable,str(CAPTURE),str(prefix)+'.source.json'],cwd=ROOT,check=True)
    subprocess.run([sys.executable,str(BASE/'providers.py'),str(prefix)+'.providers.json'],cwd=ROOT,check=True)
    return {'source':sha(str(prefix)+'.source.json'),'providers':sha(str(prefix)+'.providers.json')}

def parse_cargo(path):
    rows=[]
    for line in Path(path).read_text().splitlines():
        if line.strip(): rows.append(json.loads(line))
    return rows

lock=None; out=None; before=None; artifacts={}; results=[]; go_sha=None; exit_code=2; last_head=None; head_samples=[]
try:
    plan=json.loads(PLAN.read_text()); rows=plan['selectors']
    require(len(rows)==15 and len({x['selector'] for x in rows})==15 and all(x['expected_tests']==1 for x in rows),'original exact 15 inventory changed')
    require(sha(BASE/'guarded-exec.py')==plan['lifecycle']['executor_sha256'],'executor changed')
    require(sha('/tmp/lsp-native-protocol-v3/plan.json')==plan['lifecycle']['predecessor_plan_sha256'],'predecessor plan changed')
    old=json.loads(Path('/tmp/lsp-native-protocol-v3/plan.json').read_text())
    expected_pins=[dict(item) for item in old['source_pins']]
    for item in expected_pins:
        if item['path']=='rust/tcl-lsp-server/src/lib.rs': item['sha256']=plan['lifecycle']['source46_lib_sha256']
    require(rows==old['selectors'] and plan['source_pins'][:-1]==expected_pins,'original selectors/source pins changed outside approved lib successor')
    require(SOURCE46.is_file() and sha(SOURCE46)==plan['lifecycle']['source46_manifest_sha256'],'source46 manifest changed')
    require(GO.is_file(),'no root GO; no lock or Cargo')
    go=json.loads(GO.read_text()); go_sha=sha(GO)
    source_view(go,go_sha,plan)
    require(CARGO.is_file(),'pinned Cargo absent')
    target_dir=Path(os.environ['CARGO_TARGET_DIR']).resolve()
    disk((ROOT,'/tmp',target_dir,RUNS))
    lock=open(LOCK,'a')
    try: fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError: raise RuntimeError('STOP: build lock occupied; no queue')
    stamp=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    out=RUNS/stamp; out.mkdir(parents=True,exist_ok=False)
    save(out/'plan.json',plan); save(out/'go.json',go)
    save(out/'build-environment.json',{'required':{key:os.environ.get(key) for key in plan['required_environment']},'path':os.environ.get('PATH'),'cargo_sha256':sha(CARGO),'rustc_sha256':sha(plan['rustc_path']),'refused_overrides':environment_override_keys(plan,os.environ)})
    before=snapshot(out/'before')
    spec=importlib.util.spec_from_file_location('lsp_guarded_exec',BASE/'guarded-exec.py')
    require(spec is not None and spec.loader is not None,'executor loader missing')
    module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    cap=20000000
    for target,build_args in [('e2e',['--test','e2e']),('server_lib',['--lib'])]:
        source_view(go,go_sha,plan); require(snapshot(out/(target+'-prebuild'))==before,'source/provider changed before build')
        disk((ROOT,'/tmp',target_dir,out))
        prefix=out/('build-'+target)
        argv=[str(CARGO),'test','--locked','-p','tcl-lsp-server',*build_args,'--no-run','--message-format=json']
        save(str(prefix)+'.argv.json',argv)
        rec=module.run(argv,cwd=ROOT,env=os.environ.copy(),stdout_path=str(prefix)+'.stdout',stderr_path=str(prefix)+'.stderr',receipt_path=str(prefix)+'.process.json',artifact_path=str(CARGO),deadline_seconds=3600,cap_bytes=cap,disk_floor_bytes=2300000000,disk_paths=(ROOT,'/tmp',target_dir,out))
        require(rec['complete'] and rec['stop_reason'] is None and rec['exit_code']==0 and rec['group_empty_before_parent_reap'],'guarded Cargo incomplete')
        events=parse_cargo(str(prefix)+'.stdout')
        finished=[e for e in events if e.get('reason')=='build-finished']
        require(len(finished)==1 and finished[0].get('success'),'current Cargo build-finished missing/failure')
        warnings=[e for e in events if e.get('reason')=='compiler-message' and e.get('message',{}).get('level') in ('warning','error')]
        build_script_warnings=[e for e in events if e.get('reason')=='build-script-executed' and e.get('warnings')]
        stderr_warnings=[line for line in Path(str(prefix)+'.stderr').read_text(errors='replace').splitlines() if re.match(r'^warning:',line)]
        save(str(prefix)+'.warnings.json',{'compiler_messages':warnings,'build_script_warnings':build_script_warnings,'stderr_warning_lines':stderr_warnings})
        require(not warnings and not build_script_warnings and not stderr_warnings,'Cargo warning/error gate')
        name='e2e' if target=='e2e' else 'tcl_lsp_server'
        kind=['test'] if target=='e2e' else ['lib']
        candidates=[e for e in events if e.get('reason')=='compiler-artifact' and e.get('target',{}).get('name')==name and e.get('target',{}).get('kind')==kind and e.get('profile',{}).get('test') is True and e.get('executable') and Path(e.get('manifest_path','')).resolve()==ROOT/'rust/tcl-lsp-server/Cargo.toml']
        require(len(candidates)==1,'fresh exact Cargo test artifact absent/ambiguous')
        servers=[e for e in events if e.get('reason')=='compiler-artifact' and e.get('target',{}).get('name')=='tcl-lsp-server' and e.get('target',{}).get('kind')==['bin'] and e.get('profile',{}).get('test') is False and e.get('executable') and Path(e.get('manifest_path','')).resolve()==ROOT/'rust/tcl-lsp-server/Cargo.toml']
        require(target!='e2e' or len(servers)==1,'fresh server executable absent/ambiguous')
        exe=Path(candidates[0]['executable']).resolve()
        require(exe.is_file() and exe.is_relative_to(target_dir),'test executable outside actual target')
        server=Path(servers[0]['executable']).resolve() if servers else None
        if target=='e2e': require(server is not None and server.is_file() and server.is_relative_to(target_dir),'server executable outside actual target')
        artifacts[target]={'test':str(exe),'test_sha256':sha(exe),'server':str(server) if server else None,'server_sha256':sha(server) if server else None,'event':candidates[0]}
        save(str(prefix)+'.artifact.json',artifacts[target])
        source_view(go,go_sha,plan); require(snapshot(out/(target+'-postbuild'))==before,'source/provider changed during build')
        for index,row in enumerate(x for x in rows if (x['target']=='--lib')==(target=='server_lib')):
            source_view(go,go_sha,plan); require(snapshot(out/(target+'-pretest-'+str(index)))==before,'source/provider changed before selector')
            disk((ROOT,'/tmp',target_dir,out))
            require(sha(exe)==artifacts[target]['test_sha256'],'test artifact changed before selector')
            if server: require(sha(server)==artifacts[target]['server_sha256'],'server artifact changed before selector')
            env=os.environ.copy(); env['NEXTEST_BIN_EXE_tcl-lsp-server']=str(server) if server else ''
            label=f'{target}-{index:02}'
            prefix=out/label
            argv=[str(exe),row['selector'],'--exact','--nocapture','--test-threads=1']
            rec=module.run(argv,cwd=ROOT,env=env,stdout_path=str(prefix)+'.stdout',stderr_path=str(prefix)+'.stderr',receipt_path=str(prefix)+'.process.json',artifact_path=str(exe),deadline_seconds=180,cap_bytes=cap,disk_floor_bytes=2300000000,disk_paths=(ROOT,'/tmp',target_dir,out))
            text=Path(str(prefix)+'.stdout').read_text(errors='replace')+'\n'+Path(str(prefix)+'.stderr').read_text(errors='replace')
            started_counts=[int(value) for value in re.findall(r'^running (\d+) tests?$',text,re.M)]
            printed_names=re.findall(r'^test ([^ ]+) \.\.\.',text,re.M)
            require(printed_names==[row['selector']],'exact printed original selector mismatch')
            summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M)
            require(started_counts==[1] and len(summaries)==1,'exact one original test start/summary missing')
            status,passed_count,failed_count,ignored_count=summaries[0]
            passed_count,failed_count,ignored_count=map(int,(passed_count,failed_count,ignored_count))
            require(passed_count+failed_count==1 and ignored_count==0,'exact selected count changed')
            require((status,passed_count,failed_count,rec['exit_code']) in [('ok',1,0,0),('FAILED',0,1,101)],'original test status/exit disagreement')
            test_warnings=bool(re.search(r'^warning:',text,re.M))
            passed=(rec['complete'] and rec['stop_reason'] is None and rec['exit_code']==0 and rec['group_empty_before_parent_reap'] and rec['timer_armed_before_spawn'] and sha(exe)==artifacts[target]['test_sha256'] and (not server or sha(server)==artifacts[target]['server_sha256']) and 'running 1 test' in text and 'test result: ok. 1 passed; 0 failed' in text and not test_warnings)
            result={'selector':row['selector'],'expected_tests':1,'started_counts':started_counts,'printed_names':printed_names,'actual_selected':passed_count+failed_count,'actual_passed':passed_count,'actual_failed':failed_count,'passed':passed,'test_warnings':test_warnings,'process':rec,'test_artifact_sha256':sha(exe),'server_artifact_sha256':sha(server) if server else None,'stdout_sha256':sha(str(prefix)+'.stdout'),'stderr_sha256':sha(str(prefix)+'.stderr')}
            results.append(result); save(str(prefix)+'.result.json',result); save(out/'results.json',results)
            source_view(go,go_sha,plan); require(snapshot(out/(target+'-posttest-'+str(index)))==before,'source/provider changed after selector')
            require(rec['complete'] and rec['stop_reason'] is None and rec['group_empty_before_parent_reap'] and rec['timer_armed_before_spawn'],'selector lifecycle guard incomplete: '+row['selector'])
            require(sha(exe)==artifacts[target]['test_sha256'] and (not server or sha(server)==artifacts[target]['server_sha256']),'selector artifact changed: '+row['selector'])
    require(len(results)==15 and {x['selector'] for x in results}=={x['selector'] for x in rows},'exact 15 result inventory mismatch')
    source_view(go,go_sha,plan); require(snapshot(out/'after')==before,'final source/provider changed')
    require(all(sha(a['test'])==a['test_sha256'] and (not a['server'] or sha(a['server'])==a['server_sha256']) for a in artifacts.values()),'final artifact drift')
    all_passed=all(x['passed'] for x in results)
    source_view(go,go_sha,plan); save(out/'head-samples.json',head_samples)
    save(out/'summary.json',{'approved_product_head':plan['approved_product_head'],'approved_product_tree':plan['approved_product_tree'],'actual_head_after_final_seals':last_head,'passed':all_passed,'count':15,'source_provider_stable':True,'artifact_stable':True,'go_sha256':go_sha,'finished_utc':utc()})
    print(('PASS' if all_passed else 'FAIL')+' exact15 '+str(out),flush=True)
    exit_code=0 if all_passed else 1
except BaseException as error:
    print('STOP '+repr(error),file=sys.stderr,flush=True)
    if out is not None:
        final={'passed':False,'reason':repr(error),'results_observed':len(results),'finished_utc':utc(),'go_sha256':go_sha}
        try:
            final['after_snapshot']=snapshot(out/'failure-after')
            final['source_provider_stable']=before is not None and final['after_snapshot']==before
            final['artifact_stable']=all(sha(a['test'])==a['test_sha256'] and (not a['server'] or sha(a['server'])==a['server_sha256']) for a in artifacts.values())
        except Exception as guard_error:
            final['final_guard_incomplete']=repr(guard_error)
            final['source_provider_stable']=False; final['artifact_stable']=False
        save(out/'head-samples.json',head_samples)
        save(out/'failure.json',final)
    exit_code=2
finally:
    if lock is not None:
        try:
            fcntl.flock(lock,fcntl.LOCK_UN); lock.close()
            if out is not None: save(out/'lock-released.json',{'utc':utc(),'pid':os.getpid(),'released':True})
        except Exception as release_error:
            if out is not None: save(out/'lock-release-incomplete.json',{'utc':utc(),'reason':repr(release_error),'released':False})
            exit_code=2
raise SystemExit(exit_code)
