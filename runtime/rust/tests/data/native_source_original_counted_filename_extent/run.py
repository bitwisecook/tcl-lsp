#!/usr/bin/env python3
"""Replay to a fresh absolute directory after whole original input verification."""
from pathlib import Path
import argparse,hashlib,json
p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);args=p.parse_args();here=Path(__file__).resolve().parent;out=args.output
assert out.is_absolute()and not out.exists()and not out.is_relative_to(here)and not here.is_relative_to(out),'A fresh absolute output is required'
def sha(path):
 h=hashlib.sha256()
 with path.open('rb')as f:
  for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
 return h.hexdigest()
for name,h in json.loads((here/'pinned-input-map.json').read_text())['original_path_to_sha256'].items():assert Path(name).is_file()and sha(Path(name))==h,('Restore exact input separately',name)
for original in ['root-native-source-filename648','root-native-source-filename-codec650','root-native-source-filename-strict651']:
 original=Path('/workspace/.proofs')/original;assert not out.is_relative_to(original)and not original.is_relative_to(out)
out.mkdir()
for collection in json.loads((here/'REPLAY-PATH-ADAPTATION.json').read_text())['collections']:
 number=collection['collection'];scope={'__name__':'__main__','__replay_request':str(here/number),'__replay_output':str(out/number),'__replay_version_source':str(here/number/'library-version.c')}
 for row in collection['scripts']:
  path=here/row['path'];assert sha(path)==row['original_sha256'];text=path.read_text()
  for change in row['literal_replacements']:assert text.count(change['old'])==1;text=text.replace(change['old'],change['new'],1)
  scope['__file__']=str(path);exec(compile(text,str(path),'exec'),scope)
print(json.dumps({'new_capture':str(out),'scope':'New experiments; original captures remain unchanged and retain their own executable attribution.'}))
