#!/usr/bin/env python3
"""Exact LSP startup/UPF 4; immutable preparation, root GO required."""
from pathlib import Path
import datetime, fcntl, hashlib, importlib.util, json, os, re, shutil, subprocess, sys

ROOT=Path('/workspace/naming-migration-active/tcl-lsp')
BASE=Path('/tmp/vm26-latebound-template-v2')
PLAN=BASE/'plan.json'
GO=BASE/'root-go.json'
MANIFEST=BASE/'runner-input-manifest.json'
SOURCE122=Path('/tmp/naming-reviewed-product-actual-identity-v11/actual-source122-manifest.json')
CAPTURE=ROOT/'docs/work/name-handling/validation-tools/capture-inputs.py'
LOCK=Path('/tmp/tcl-lsp-naming-build.lock')
CARGO=Path('/workspace/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/cargo')
RUNS=Path('/workspace/naming-validation-runs/vm26-latebound-template-v2')

def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()
def unique_json(pairs):
    value={}
    for key,child in pairs:
        if key in value:raise RuntimeError('STOP: duplicate JSON key '+key)
        value[key]=child
    return value
def read_json_text(text):return json.loads(text,object_pairs_hook=unique_json)
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
    require(go.get('source122_manifest_sha256')==sha(SOURCE122)==plan['lifecycle']['source122_manifest_sha256'],'source122 manifest pin mismatch')
    identity=plan['actual_identity']
    require(sha(identity['path'])==identity['sha256'],'actual source122 identity changed')
    current_identity=read_json_text(Path(identity['path']).read_text())
    require(current_identity.get('approved_integrated_head')==plan['approved_product_head'] and current_identity.get('approved_integrated_tree')==plan['approved_product_tree'] and current_identity.get('path_count')==plan['source_path_count'],'actual identity product/count mismatch')
    typing_plan=plan['typecheck_plan']
    require(sha(typing_plan['path'])==typing_plan['sha256'] and sha(typing_plan['runner_inputs_path'])==typing_plan['runner_inputs_sha256'],'current typecheck protocol changed')
    for path,pin in json.loads(Path(typing_plan['runner_inputs_path']).read_text())['inputs'].items():
        require(sha(path)==pin,'typecheck transitive input changed: '+path)
    approved=go.get('approved_integrated_head')
    require(isinstance(approved,str) and re.fullmatch(r'[0-9a-f]{40}',approved),'missing exact approved HEAD')
    require(approved==plan['approved_product_head'],'GO differs from actual integrated source122 product')
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
    for path,expected in read_json_text(MANIFEST.read_text())['inputs'].items(): require(sha(path)==expected,'runner input changed: '+path)
    for item in plan['source_pins']: require(sha(ROOT/item['path'])==item['sha256'],'source pin changed: '+item['path'])
    source_rows=read_json_text(SOURCE122.read_text())['paths']
    require(len(source_rows)==plan['source_path_count'] and len({row['path'] for row in source_rows})==plan['source_path_count'],'source122 inventory changed')
    for row in source_rows:
        path=row['path']; expected=row['after_sha256']
        require(sha(ROOT/path)==expected,'source122 final hash changed: '+path)
        committed=subprocess.run(['git','show',approved+':'+path],cwd=ROOT,capture_output=True)
        require(committed.returncode==0 and hashlib.sha256(committed.stdout).hexdigest()==expected,'approved product source122 blob mismatch: '+path)
    require(subprocess.run(['git','diff','--quiet',approved,'--','.',':(exclude)docs/work/name-handling'],cwd=ROOT).returncode==0,'worktree product differs from approved HEAD/tree')
    for key in ('post122_eight_package_typecheck_evidence',):
        evidence=go.get(key)
        require(isinstance(evidence,dict) and evidence.get('path') and evidence.get('sha256') and sha(evidence['path'])==evidence['sha256'],'gate evidence missing/changed: '+key)
    typing=read_json_text(Path(go['post122_eight_package_typecheck_evidence']['path']).read_text())
    require(typing.get('status')=='PASS' and typing.get('overall_exit')==0 and typing.get('cargo_exit_code')==0,'eight-package check not PASS0')
    require(all(typing.get(key) is True for key in ('guard_valid','warning_free_json_stderr_stdout','source_stable','providers_stable','root_go_stable','head_docs_descendant_valid')),'eight-package check guard/warning seal incomplete')
    require(typing.get('plan_sha256')==plan['lifecycle']['post122_eight_package_typecheck_plan_sha256'],'wrong eight-package check plan')
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
    require(min(result.values())>=2*1024**3,'actual mount below 2GiB floor: '+str(result))
    return result

def snapshot(prefix):
    subprocess.run([sys.executable,str(CAPTURE),str(prefix)+'.source.json'],cwd=ROOT,check=True)
    subprocess.run([sys.executable,str(BASE/'providers.py'),str(prefix)+'.providers.json'],cwd=ROOT,check=True)
    return {'source':sha(str(prefix)+'.source.json'),'providers':sha(str(prefix)+'.providers.json')}

def parse_cargo(path):
    rows=[]
    for line in Path(path).read_text().splitlines():
        if line.strip(): rows.append(read_json_text(line))
    return rows

lock=None; out=None; before=None; artifacts={}; results=[]; go_sha=None; exit_code=2; last_head=None; head_samples=[]; remaining=20*1024**2
try:
    plan=read_json_text(PLAN.read_text()); rows=plan['selectors']; builds=plan['package_builds']
    require(len(rows)==26 and len({(x['artifact_key'],x['selector']) for x in rows})==26 and all(x['expected_tests']==1 for x in rows),'exact exact26 inventory changed')
    require(len(builds)==11 and len({x['artifact_key'] if 'artifact_key' in x else x['package']+':test:'+x['target'] for x in builds})==11,'eleven target inventory changed')
    require(sha(BASE/'guarded-exec.py')==plan['lifecycle']['executor_sha256'],'executor changed')
    require(GO.is_file(),'no root GO; no lock or Cargo')
    go=read_json_text(GO.read_text()); go_sha=sha(GO)
    source_view(go,go_sha,plan)
    target_dir=Path(os.environ['CARGO_TARGET_DIR']).resolve(); disk((ROOT,'/tmp',target_dir,RUNS))
    lock=open(LOCK,'a'); fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    stamp=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ'); out=RUNS/stamp; out.mkdir(parents=True,exist_ok=False)
    save(out/'plan.json',plan); save(out/'go.json',go)
    save(out/'build-environment.json',{'required':{key:os.environ.get(key) for key in plan['required_environment']},'path':os.environ.get('PATH'),'refused_overrides':environment_override_keys(plan,os.environ)})
    before=snapshot(out/'before')
    spec=importlib.util.spec_from_file_location('vm16_guarded_exec',BASE/'guarded-exec.py'); require(spec is not None and spec.loader is not None,'executor loader missing')
    module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    def execute(argv,prefix,artifact,seconds,env):
        global remaining
        require(remaining>0,'aggregate log cap exhausted before spawn')
        rec=module.run(argv,cwd=ROOT,env=env,stdout_path=str(prefix)+'.stdout',stderr_path=str(prefix)+'.stderr',receipt_path=str(prefix)+'.process.json',artifact_path=str(artifact),deadline_seconds=seconds,cap_bytes=remaining,disk_floor_bytes=2*1024**3,disk_paths=(ROOT,'/tmp',target_dir,out))
        retained=rec.get('retained_bytes'); require(isinstance(retained,int) and 0<=retained<=remaining,'log accounting invalid')
        remaining-=retained;save(out/'aggregate-log-budget.json',{'cap_bytes':20*1024**2,'remaining_bytes':remaining})
        require(remaining>0 and rec['complete'] and rec['stop_reason'] is None and rec['group_empty_before_parent_reap'] and rec['timer_armed_before_spawn'] and rec['artifact_unchanged'],'owned process lifecycle/artifact/log guard incomplete')
        return rec
    for build in builds:
        package=build['package']; target=build['target']; key=package+':test:'+target; label=package+'-'+target
        source_view(go,go_sha,plan);disk((ROOT,'/tmp',target_dir,out))
        prefix=out/('build-'+label);argv=build['cargo_argv'];require(build['features']==(['all-features'] if package=='tcl-vm' else []),'feature contract changed');require(argv==[str(CARGO),'test','--locked','-p',package,'--test',target]+(['--all-features'] if package=='tcl-vm' else [])+['--no-run','--message-format=json'],'original Cargo flags changed');save(str(prefix)+'.argv.json',argv)
        rec=execute(argv,prefix,CARGO,3600,os.environ.copy());require(rec['exit_code']==0,'Cargo build failed')
        events=parse_cargo(str(prefix)+'.stdout');finished=[x for x in events if x.get('reason')=='build-finished'];require(len(finished)==1 and finished[0].get('success') is True,'Cargo terminal build event missing')
        warnings=[x for x in events if x.get('reason')=='compiler-message' and x.get('message',{}).get('level') in ('warning','error')];script_warnings=[x for x in events if x.get('reason')=='build-script-executed' and x.get('warnings')];stderr_warnings=re.findall(r'^warning:.*',Path(str(prefix)+'.stderr').read_text(errors='replace'),re.M)
        save(str(prefix)+'.warnings.json',{'compiler':warnings,'scripts':script_warnings,'stderr':stderr_warnings});require(not warnings and not script_warnings and not stderr_warnings,'strict0W build gate')
        candidates=[x for x in events if x.get('reason')=='compiler-artifact' and x.get('target',{}).get('name')==target and x.get('target',{}).get('kind')==['test'] and x.get('profile',{}).get('test') is True and x.get('executable') and Path(x.get('manifest_path','')).resolve()==Path(build['manifest_path'])]
        require(len(candidates)==1,'fresh exact Cargo artifact absent/ambiguous');event=candidates[0];exe=Path(event['executable']).resolve();require(exe.is_file() and exe.is_relative_to(target_dir),'artifact outside actual target')
        artifacts[key]={'path':str(exe),'sha256':sha(exe),'event':event};save(str(prefix)+'.artifact.json',artifacts[key])
        source_view(go,go_sha,plan);require(sha(GO)==go_sha,'GO drift during build')
        roster_prefix=out/('roster-'+label);roster=execute([str(exe),'--list','--format','terse'],roster_prefix,exe,180,os.environ.copy());require(roster['exit_code']==0,'libtest roster failed')
        names=re.findall(r'^(.+): test$',Path(str(roster_prefix)+'.stdout').read_text(errors='replace'),re.M);require(names,'empty libtest roster')
        selected=[x for x in rows if x['artifact_key']==key];require(selected,'unexpected target without selectors')
        for row in selected:require(names.count(row['selector'])==1,'exact selector absent/duplicate in fresh roster: '+row['selector'])
        save(str(roster_prefix)+'.selected.json',{'roster':names,'expected':[x['selector'] for x in selected]})
        for row in selected:
            source_view(go,go_sha,plan);disk((ROOT,'/tmp',target_dir,out));require(sha(exe)==artifacts[key]['sha256'],'artifact drift before test')
            prefix=out/(f'{len(results)+1:02d}-'+row['selector']);env=os.environ.copy();env.update(row.get('env',{}));argv=[str(exe),row['selector'],'--exact','--nocapture','--test-threads=1'];save(str(prefix)+'.argv.json',argv);save(str(prefix)+'.env.json',row.get('env',{}));rec=execute(argv,prefix,exe,180,env)
            text=Path(str(prefix)+'.stdout').read_text(errors='replace')+'\n'+Path(str(prefix)+'.stderr').read_text(errors='replace');started=[int(x) for x in re.findall(r'^running (\d+) tests?$',text,re.M)];printed=re.findall(r'^test ([^ ]+) \.\.\.',text,re.M);summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M)
            require(started==[1] and printed==[row['selector']] and len(summaries)==1,'exact libtest start/name/summary count mismatch');status,passed,failed,ignored=summaries[0];passed,failed,ignored=map(int,(passed,failed,ignored));require(ignored==0 and (status,passed,failed,rec['exit_code']) in [('ok',1,0,0),('FAILED',0,1,101)],'libtest summary/status/exit mismatch');require(not re.search(r'^warning:',text,re.M),'runtime warning violates0W')
            source_view(go,go_sha,plan);require(sha(exe)==artifacts[key]['sha256'],'artifact drift after test')
            result={'index':len(results)+1,'selector':row['selector'],'artifact_key':key,'actual_selected':passed+failed,'actual_passed':passed,'actual_failed':failed,'passed':passed==1,'summaries':summaries,'started_counts':started,'printed_names':printed,'process':rec,'stdout_sha256':sha(str(prefix)+'.stdout'),'stderr_sha256':sha(str(prefix)+'.stderr'),'source_provider_stable':None,'provisional_until_final_seals':True,'artifact_stable':True,'warnings':False};results.append(result);save(str(prefix)+'.result.json',result);save(out/'results.json',results);print('RESULT',len(results),row['selector'],'PASS' if passed else 'FAIL',flush=True)
    require(len(results)==26 and len(artifacts)==11 and sum(x['actual_selected'] for x in results)==26,'terminal26/11 inventory mismatch')
    source_view(go,go_sha,plan);require(snapshot(out/'after')==before,'final source/provider drift');require(all(sha(x['path'])==x['sha256'] for x in artifacts.values()),'final artifact drift');source_view(go,go_sha,plan);save(out/'head-samples.json',head_samples)
    save(out/'final.json',{'approved_product_head':plan['approved_product_head'],'approved_product_tree':plan['approved_product_tree'],'actual_head_after_final_seals':last_head,'count':26,'artifacts':artifacts,'PASS':sum(x['passed'] for x in results),'FAIL':sum(not x['passed'] for x in results),'source_provider_stable':True,'artifact_stable':True,'warnings':False,'go_sha256':go_sha,'finished_utc':utc()});exit_code=0 if all(x['passed']for x in results) else 1;print('COMPLETE exact26 overall='+str(exit_code)+' receipt='+str(out),flush=True)
except BaseException as error:
    print('STOP '+repr(error),file=sys.stderr,flush=True)
    if out is not None:
        failure={'protocol_failure':True,'reason':repr(error),'results_observed':len(results),'finished_utc':utc(),'go_sha256':go_sha}
        try:failure['after_snapshot']=snapshot(out/'failure-after');failure['source_provider_stable']=before is not None and failure['after_snapshot']==before;failure['artifact_stable']=all(sha(x['path'])==x['sha256'] for x in artifacts.values())
        except Exception as guard_error:failure['final_guard_incomplete']=repr(guard_error)
        save(out/'failure.json',failure)
    exit_code=2
finally:
    if lock is not None:
        try:
            fcntl.flock(lock,fcntl.LOCK_UN);lock.close()
            if out is not None:save(out/'lock-released.json',{'utc':utc(),'pid':os.getpid(),'released':True})
            print('LOCK_RELEASED '+utc(),flush=True)
        except Exception as release_error:
            if out is not None:save(out/'lock-release-incomplete.json',{'reason':repr(release_error),'released':False})
            exit_code=2
raise SystemExit(exit_code)
