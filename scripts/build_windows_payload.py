#!/usr/bin/env python3
import argparse,base64,json,shutil
from pathlib import Path
root=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
a.out.mkdir(parents=True,exist_ok=True);app=a.out/'app';app.mkdir(exist_ok=True)
shutil.copy2(a.binary,app/'kedr-studio.exe')
for name in ['README.md','LICENSE']:shutil.copy2(root/name,app/name)
for name in ['docs','products','assets/brand']:shutil.copytree(root/name,app/name,dirs_exist_ok=True)
shutil.copy2(root/'LICENSE',a.out/'LICENSE')
icons=json.loads((root/'packaging/assets/codecedar.json').read_text());(a.out/'codecedar.ico').write_bytes(base64.b64decode(icons['codecedar.ico'],validate=True))
files=[f.relative_to(app).as_posix().replace('/','\\') for f in app.rglob('*') if f.is_file()]
# Delete only shipped files/directories, never RMDir /r the installation root.
lines=['Delete "$INSTDIR\\'+f+'"' for f in files]
folders=sorted([f.relative_to(app).as_posix().replace('/','\\') for f in app.rglob('*') if f.is_dir()],key=len,reverse=True)
lines+=['RMDir "$INSTDIR\\'+f+'"' for f in folders]
(a.out/'uninstall-files.nsh').write_text('\n'.join(lines)+'\n')
checks=['IfFileExists "$INSTDIR\\kedr-owned.txt" destination_owned']
for n,f in enumerate(files+['codecedar.ico','Uninstall.exe']):
 checks += [f'IfFileExists "$INSTDIR\\{f}" 0 collision_{n}_clear','DetailPrint "Refusing to overwrite an unowned existing installation file"','SetErrorLevel 2','Abort',f'collision_{n}_clear:']
checks+=['destination_owned:']
(a.out/'check-destination.nsh').write_text('\n'.join(checks)+'\n')
print('Prepared Windows payload:',len(files),'managed files')
