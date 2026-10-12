from pathlib import Path
import json,hashlib,re
base=Path(__file__).resolve().parent;sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r=json.loads((base/'request.json').read_text());r['launcher_sha256']=sha(base/'launch.py');r['required_sha256'][str(base/'launch.py')]=sha(base/'launch.py')
r['reused_authentic_harnesses']=[dict(path='/tmp/receiver-oo-operand-stages-request499/probe.c',sha256=sha('/tmp/receiver-oo-operand-stages-request499/probe.c'),purpose='Genuine original lazy ObjType update/free/duplicate and raw-header/refcount observer; C9 length ABI separated from legacy command callback.'),dict(path='/workspace/.proofs/native-naming-argument-order499d/launch.py',sha256=sha('/workspace/.proofs/native-naming-argument-order499d/launch.py'),purpose='Independent fresh compile/link/dependency/process receipts and six original provider/source closure. No earlier rows copied as new observations.')]
windows=[]
for b in r['provider_builds']:
 provider=b['provider'];files=[p for p in b['required_sha256']if p.endswith('tclInterp.c'if provider!='jim'else'jim.c')]
 for path in files:
  lines=Path(path).read_text().splitlines();names=['AliasCreate','AliasDescribe','AliasObjCmd','AliasObjCmdDeleteProc','TclPreventAliasLoop']if provider!='jim'else['JimAliasCmd','JimAliasCmdDelete','Jim_AliasCoreCommand']
  if provider in ['9.0.4','9.1.0']:names=['TclAliasCreate','AliasDescribe','TclAliasObjCmd','TclLocalAliasObjCmd','AliasObjCmdDeleteProc','TclPreventAliasLoop']
  for name in names:
   hits=[i for i,x in enumerate(lines)if re.match(r'(?:static\s+(?:int|void)\s+)?'+re.escape(name)+r'\(',x)]
   assert hits,(provider,name)
   i=hits[-1];a=max(0,i-6);z=min(len(lines),i+150);w=base/'source-windows'/provider/(name+'.txt');w.parent.mkdir(parents=True,exist_ok=True);w.write_text('\n'.join(f'{k+1}: {lines[k]}'for k in range(a,z))+'\n');windows.append(dict(provider=provider,path=path,source_sha256=sha(path),anchor=name,line=i+1,window=str(w),window_sha256=sha(w)))
r['source_windows']=windows
for x in windows:r['required_sha256'][x['window']]=x['window_sha256']
r['static_review']=dict(reviewer='/root/receiver_finish',scope='Original live references/query member access/count ABI/independent Jim purpose; no concrete blocker. Metadata receipt filter corrected to retain actual VERSION/BUILD/API lines.',status='Bounded read-only source review; not compiled or executed')
(base/'request.json').write_text(json.dumps(r,indent=2)+'\n')
(base/'README.md').write_text('''# Original alias object probe request

The probe enters the original public object-vector command interface with uncached custom object headers. It records actual String updater, internal-representation free and duplicate callbacks; raw rows inspect only live held headers, refCounts, type/cache presence and actual List members. A free callback records internal representation retirement, not an inferred object-header destruction.

Cases 1–5 retain the original alias name, target and two prefixes. Two raw query owners are held concurrently where requested. Query List presentation is a separate operation. The registered target compares received original headers and intentionally reads the second prefix before the first; a second invocation exposes actual cache reuse. Caller references, actual alias ownership and query List references are retired at separate steps. A counted NUL and spaces remain in the first original prefix. Case 6 separately enters a genuine C child alias targeting the parent. The current Jim core alias/info-alias path is independent; its different interpreter command spelling is observed, and no C-style cross-interpreter List-query API or pointer recipe is manufactured.

`request.json` pins the exact probe, launcher, original provider headers, static libraries, compiler/link inputs, runtime dependencies and source files. `launch.py` refuses an existing output directory, verifies inputs before and after each process, associates each actual ELF with its own successful compile, retains full output/status/timeout and records actual BUILD/VERSION/API rows. Root alone launches it. Six closed case records per provider mean process capture closure, never six semantic PASS answers. Individual Guest statuses and Jim unavailability remain in the raw stream.

The C source inspection includes alias capture/publication, fresh query List, actual invocation and delete ownership, and the post-publication original target getter in TclPreventAliasLoop. TclOO same-header observations supply no alias observation. No Rust implementation or assertion result follows from this prepared request.
''')
artifacts=[base/'probe.c',base/'launch.py',base/'request.json',base/'README.md',base/'seal.py']+[Path(w['window'])for w in windows]
manifest=dict(packet='ConsumerOriginalAliasObjectProbe415',status='Launch-ready request only; not compiled, executed or interpreted as provider PASS.',request=dict(path=str(base/'request.json'),sha256=sha(base/'request.json')),artifacts=[dict(path=str(p),sha256=sha(p))for p in artifacts],providers=r['providers'],pinned_input_count=len(r['required_sha256']),scope='Original capture/query List header/member identity, selected invocation getter order/cache and deletion reference ownership. C and Jim purpose separated.',static_review=r['static_review'])
(base/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
for p,h in r['required_sha256'].items():assert sha(p)==h,p
assert not(base/'providers').exists()
print(json.dumps(dict(manifest=str(base/'manifest.json'),manifest_sha256=sha(base/'manifest.json'),request_sha256=sha(base/'request.json'),probe_sha256=sha(base/'probe.c'),launch_sha256=sha(base/'launch.py'),source_windows=len(windows),pinned_inputs=len(r['required_sha256'])),indent=2))
