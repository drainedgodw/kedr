#!/usr/bin/env python3
import argparse,base64,hashlib,json,shutil,platform
from pathlib import Path
root=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
a.out.mkdir(parents=True,exist_ok=True);(a.out/'payload').mkdir(exist_ok=True)
shutil.copy2(a.binary,a.out/'payload/kedr-studio');(a.out/'payload/kedr-studio').chmod(0o755)
icons=json.loads((root/'packaging/assets/codecedar.json').read_text())
for name,data in icons.items():(a.out/'payload'/name).write_bytes(base64.b64decode(data,validate=True))
shutil.copy2(root/'assets/brand/codecedar.svg',a.out/'payload/codecedar.svg')
for src in ['installer.py','install.sh']:shutil.copy2(root/'packaging/linux'/src,a.out/src)
(a.out/'install.sh').chmod(0o755)
shutil.copy2(root/'LICENSE',a.out/'LICENSE');shutil.copy2(root/'docs/suite/INSTALL.md',a.out/'README.md')
products=json.loads((root/'products/catalog.json').read_text())['products'];records=[]
for item in products:
 record={k:item[k] for k in ['slug','name','status']};record['available']=item['slug']=='codecedar' and item['status']=='preview'
 if record['available']:
  paths={'binary':'kedr-studio','svg':'codecedar.svg',**{f'png-{s}':f'codecedar-{s}.png' for s in [48,128,256,512]}}
  record['payload']={key:{'path':'payload/'+name,'sha256':hashlib.sha256((a.out/'payload'/name).read_bytes()).hexdigest()} for key,name in paths.items()}
 records.append(record)
(a.out/'install-manifest.json').write_text(json.dumps({'schema':1,'platform':'Linux','arch':platform.machine(),'version':'0.1.0-preview','products':records},indent=2)+'\n')
print('Built offline bundle:',a.out)
