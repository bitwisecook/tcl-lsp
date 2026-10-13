import pathlib,shlex,json

def selected_output_coverage(c, artifacts, packages, before, after):
 by_id={p['id']:p for p in packages.values()}
 bindings=[];unbound=[];unresolved=[];inputs={};depfiles={}
 for artifact in artifacts:
  package=by_id.get(artifact['package_id'])
  if package is None:continue
  package_root=pathlib.Path(package['manifest_path']).parent
  for name in artifact['filenames']:
   output=pathlib.Path(name)
   if output.suffix in ['.rlib','.rmeta','.so','.dylib','.dll']:stem=output.stem.removeprefix('lib');dep=output.with_name(stem+'.d')
   elif output.name=='build-script-build':
    candidates=sorted(output.parent.glob('build_script_build*.d'))
    dep=candidates[0] if len(candidates)==1 else None
   else:dep=output.with_suffix('.d')
   if dep is None or not dep.is_file():
    unbound.append({'package_id':artifact['package_id'],'artifact':name,'reason':'no uniquely bound dep-info'});continue
   bindings.append({'package_id':artifact['package_id'],'artifact':name,'dep_info':str(dep),'sha256':c.digest(dep),'fresh':artifact['fresh']})
   if str(dep) in depfiles:continue
   depfiles[str(dep)]=c.digest(dep)
   line=next((line for line in dep.read_text(errors='replace').replace('\\\n','').splitlines() if ': ' in line),'')
   if not line:
    unresolved.append({'dep_info':str(dep),'reason':'no dependency line'});continue
   try:tokens=shlex.split(line.split(': ',1)[1])
   except ValueError:
    unresolved.append({'dep_info':str(dep),'reason':'unparseable dependency line'});continue
   for token in tokens:
    path=pathlib.Path(token)
    if not path.is_absolute():
     candidates=[c.ROOT/path,package_root/path]
     path=next((p for p in candidates if p.is_file()),candidates[0])
    path=path.resolve()
    if not path.is_file():
     unresolved.append({'dep_info':str(dep),'input':token,'reason':'missing file'});continue
    inputs[c.key(path)]=c.digest(path)
 covered={};changed={};post={}
 for key,sha in sorted(inputs.items()):
  if before.get(key)==sha and after.get(key)==sha:covered[key]=sha
  elif key in before:changed[key]={'before':before[key],'after':after.get(key),'actual':sha}
  else:post[key]={'sha256':sha,'classification':'generated or dependency post-build input; not frozen before build'}
 return {'scope':'Recorded Cargo local package outputs and their bound dep-info. Authored local package trees captured before/after; third-party/generated post-only inputs retained separately. No full external dependency before/after freeze claim.','output_bindings':bindings,'dep_info':depfiles,'inputs':inputs,'covered_before_and_after':covered,'post_build_only':post,'changed_or_removed':changed,'unbound_outputs':unbound,'unresolved':unresolved}
