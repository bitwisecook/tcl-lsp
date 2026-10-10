from pathlib import Path
import json,hashlib,re
P=Path(__file__).resolve().parent;previous=json.loads(Path('/workspace/.proofs/native-mathop-constructor424/request.json').read_text())
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
providers=[];required={str(P/'probe.tcl'):h(P/'probe.tcl'),str(P/'launch.py'):h(P/'launch.py')};sources=[]
for prior in previous['provider_builds']:
 tag=prior['provider'];required.update(prior['required_sha256'])
 executable=next(k for k in prior['required_sha256'] if k.endswith('/jimsh')or k.endswith('/tclsh'))
 providers.append({'provider':tag,'command':[executable,str(P/'probe.tcl')],'executable':executable,'environment':prior['environment'],'version_receipt':prior['version_receipt'],'version_receipt_sha256':prior['version_receipt_sha256']})
 source_root=Path(executable).parent if tag=='jim' else Path(executable).parent.parent/'generic'
 anchors=[('jim.c','Jim_ErrorCoreCommand'),('jim.c','Jim_SetCoreCommand'),('jim.c','Jim_WrongNumArgs')]if tag=='jim' else [('tclCmdAH.c','Tcl_ErrorObjCmd'),('tclVar.c','Tcl_SetObjCmd'),('tclIndexObj.c','Tcl_WrongNumArgs')]
 for file,anchor in anchors:
  source=source_root/file
  if not source.exists():raise RuntimeError(str(source))
  raw=source.read_bytes();lines=raw.splitlines(keepends=True)
  # Select the actual definition, not a command table or forward prototype.
  hits=[i for i,line in enumerate(lines)if anchor.encode() in line and (line.startswith(anchor.encode()+b'(')or line.startswith(b'static int '+anchor.encode()+b'(')or line.startswith(b'void '+anchor.encode()+b'('))]
  if not hits:
   # Jim uses a line break between return type and definition; its column-zero name owns that definition.
   hits=[i for i,line in enumerate(lines)if line.startswith(anchor.encode()+b'(')]
  if len(hits)!=1:raise RuntimeError((str(source),anchor,hits))
  first=max(0,hits[0]-12);last=min(len(lines),hits[0]+105)
  window=P/'source-windows'/tag/(file+'.'+anchor+'.txt');window.parent.mkdir(parents=True,exist_ok=True);window.write_bytes(b''.join(lines[first:last]));required[str(source)]=h(source);required[str(window)]=h(window)
  sources.append({'provider':tag,'source':str(source),'source_sha256':h(source),'anchor':anchor,'first_line':first+1,'last_line':last,'window_file':str(window),'window_sha256':h(window)})
request={'question_id':'naming.diagnostics.original-error-code-metadata-not-message','document':'docs/design/analysis/name-resolution-proofs/diagnostics-original-error-code-metadata-not-message.md','fixture_root':'rust/tcl-registry/tests/data/native_error_code_metadata_original','problem_statement':'Does guest error text that looks like a wrong-arguments diagnostic select its error-code classification, or does classification belong to the original error producer and explicit metadata? Distinguish an arbitrary error message, genuine set wrongargs and explicit supplied error code independently in every original provider.','scope':'Three public original script operations in fresh C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and pinned current Jim CLI processes. Each case starts with an explicit seed in ::errorCode to distinguish replacement from an untouched global. Only actual catch status/result bytes and public errorCode variable availability/value are recorded; no private result/header/opcode/lifetime inference. VERSION_QUERY observes info patchlevel independently; unchanged prior version receipts are reused only under their exact hashes. Jim unsupported/untouched errorCode must be described as N/A for a native diagnostic owner, never promoted from a seed or message. BIG-IP untested.','probe_file':str(P/'probe.tcl'),'probe_sha256':h(P/'probe.tcl'),'launcher_file':str(P/'launch.py'),'launcher_sha256':h(P/'launch.py'),'providers':providers,'required_sha256':required,'source_windows':sources,'original_prior_pins':'All provider/archive/config/library/CLI/source/version pins retained from exact immutable Native424 request; distinct question, new original script/process streams.','status':'PREPARED; not executed by subagent; Root owns all process launches'}
(P/'request.json').write_text(json.dumps(request,indent=2)+'\n');print('prepared providers',len(providers),'input pins',len(required),'source windows',len(sources),'request SHA',h(P/'request.json'))
