#!/usr/bin/env python3
"""Offline, per-user XDG installer. Planned products are never selectable."""
import argparse,hashlib,json,os,platform,shlex,shutil,subprocess,sys,tempfile
from pathlib import Path
ID='io.github.drainedgodw.CodeCedar'
def digest(data): return hashlib.sha256(data).hexdigest()
def xdg(name, default):
    value=os.environ.get(name)
    return Path(value) if value and Path(value).is_absolute() else default
class Installer:
    def __init__(self,bundle,home=None):
        self.bundle=Path(bundle).absolute();self.home=Path(home or Path.home()).absolute()
        self.data=xdg('XDG_DATA_HOME',self.home/'.local/share')
        self.bin=self.home/'.local/bin';self.app=self.data/'kedr/codecedar'
        self.state=self.data/'kedr/install-state/codecedar.json'
        self.paths={'binary':self.app/'kedr-studio','launcher':self.bin/'codecedar','desktop':self.data/f'applications/{ID}.desktop','svg':self.data/'icons/hicolor/scalable/apps/codecedar.svg'}
        for size in [48,128,256,512]:self.paths[f'png-{size}']=self.data/f'icons/hicolor/{size}x{size}/apps/codecedar.png'
        self.manifest=json.loads((self.bundle/'install-manifest.json').read_text(encoding='utf-8'))
        if self.manifest.get('schema')!=1:raise ValueError('Unsupported bundle schema')
    def safe(self,path):
        path=Path(path)
        if any(c in str(path) for c in '\n\r\0'):raise ValueError('Control characters in installation path')
        root=self.bin if path.is_relative_to(self.bin) else self.data
        if not path.is_relative_to(root):raise ValueError('Installation path outside owned roots')
        current=path
        while True:
            if current.is_symlink():raise ValueError(f'Refusing symlink: {current}')
            if current==root:break
            current=current.parent
    def old(self):
        self.safe(self.state)
        if not self.state.exists():return {}
        data=json.loads(self.state.read_text(encoding='utf-8'))
        files=data.get('files')
        if data.get('schema')!=1 or not isinstance(files,dict) or any(k not in self.paths for k in files):raise ValueError('Invalid installed ownership record')
        if any(not isinstance(v,str) or len(v)!=64 or any(c not in '0123456789abcdef' for c in v) for v in files.values()):raise ValueError('Invalid ownership hashes')
        return files
    def payload(self,product):
        entry=next((p for p in self.manifest['products'] if p['slug']==product),None)
        if not entry or not entry.get('available'):raise ValueError('This product is planned; no installable build exists')
        if product!='codecedar':raise ValueError('Unsupported product backend')
        if self.manifest.get('platform')!='Linux' or self.manifest.get('arch') not in [platform.machine(),'any-test']:raise ValueError('Bundle platform/architecture does not match this machine')
        data={}
        for key,meta in entry['payload'].items():
            if key not in self.paths or key in ['launcher','desktop']:raise ValueError('Unexpected payload key')
            rel=Path(meta['path'])
            if rel.is_absolute() or '..' in rel.parts:raise ValueError('Unsafe bundle path')
            path=self.bundle/rel
            if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(self.bundle.resolve()):raise ValueError('Unsafe or missing payload')
            raw=path.read_bytes()
            if digest(raw)!=meta['sha256']:raise ValueError('Payload checksum mismatch')
            data[key]=raw
        if set(data)!=set(self.paths)-{'launcher','desktop'}:raise ValueError('Incomplete application/icon payload')
        data['launcher']=('#!/bin/sh\nexec '+shlex.quote(str(self.paths['binary']))+' "$@"\n').encode()
        def escape(v):return str(v).replace('\\','\\\\').replace('$','\\$').replace('`','\\`').replace('"','\\"').replace('%','%%')
        data['desktop']=(f'[Desktop Entry]\nType=Application\nName=CodeCedar\nComment=Kedr local development workspace\nExec="{escape(self.paths["launcher"])}"\nIcon=codecedar\nTerminal=false\nCategories=Development;IDE;\nStartupNotify=true\nStartupWMClass=io.github.drainedgodw.CodeCedar\n').encode()
        return data
    def install(self,product):
        data=self.payload(product);old=self.old()
        for key,path in self.paths.items():
            self.safe(path)
            if path.exists() and (key not in old or not path.is_file() or digest(path.read_bytes())!=old[key]):raise ValueError(f'Refusing to overwrite an unowned/modified file: {path}')
        changes=[]
        try:
            for key,raw in data.items():
                path=self.paths[key];path.parent.mkdir(parents=True,exist_ok=True)
                previous=path.read_bytes() if path.exists() else None
                mode=path.stat().st_mode & 0o777 if path.exists() else None
                self.atomic(path,raw,0o755 if key in ['binary','launcher'] else 0o644)
                changes.append((path,previous,mode))
            self.state.parent.mkdir(parents=True,exist_ok=True)
            self.atomic(self.state,(json.dumps({'schema':1,'version':self.manifest['version'],'files':{k:digest(v) for k,v in data.items()}},indent=2)+'\n').encode(),0o600)
        except Exception:
            for path,previous,mode in reversed(changes):
                if previous is None:path.unlink(missing_ok=True)
                else:self.atomic(path,previous,mode)
            raise
        self.refresh()
        return self.paths['launcher']
    def atomic(self,path,data,mode):
        self.safe(path);fd,tmp=tempfile.mkstemp(prefix='.kedr-',dir=path.parent)
        try:
            with os.fdopen(fd,'wb') as f:f.write(data);f.flush();os.fsync(f.fileno())
            os.chmod(tmp,mode);os.replace(tmp,path)
        finally:
            Path(tmp).unlink(missing_ok=True)
    def uninstall(self):
        old=self.old();remaining={}
        for key in old:self.safe(self.paths[key])
        for key,sha in old.items():
            path=self.paths[key];self.safe(path)
            if not path.exists():continue
            if not path.is_file() or digest(path.read_bytes())!=sha:remaining[key]=sha;continue
            path.unlink()
        if remaining:self.atomic(self.state,(json.dumps({'schema':1,'files':remaining},indent=2)+'\n').encode(),0o600)
        else:self.state.unlink(missing_ok=True)
        for path in sorted({p.parent for p in self.paths.values()},key=lambda p:len(p.parts),reverse=True):
            try:path.rmdir()
            except OSError:pass
        self.refresh();return remaining
    def refresh(self):
        # Optional desktop caches; no shell command expansion or DE-specific guesses.
        for tool,args in [('update-desktop-database',[str(self.data/'applications')]),('gtk-update-icon-cache',['-f','-t',str(self.data/'icons/hicolor')])]:
            exe=shutil.which(tool)
            if exe:
                try:subprocess.run([exe,*args],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=15,check=False)
                except (OSError,subprocess.TimeoutExpired):pass
    def verify(self):
        old=self.old()
        return bool(old) and not any(not self.paths[k].is_file() or self.paths[k].is_symlink() or digest(self.paths[k].read_bytes())!=v for k,v in old.items())
def main():
    p=argparse.ArgumentParser(description='Kedr offline installer: one available preview, planned products disabled')
    p.add_argument('--bundle',type=Path,default=Path(__file__).resolve().parent);p.add_argument('--product');p.add_argument('--list',action='store_true');p.add_argument('--uninstall',action='store_true');p.add_argument('--verify',action='store_true')
    a=p.parse_args();i=Installer(a.bundle)
    if a.list:
        for n,item in enumerate(i.manifest['products'],1):print(f'{n:2}. {item["name"]:18} '+('AVAILABLE — desktop preview' if item.get('available') else 'NOT AVAILABLE — planned'))
        return 0
    if a.uninstall:
        left=i.uninstall();print('Removed managed installation; preferences/projects are untouched.' if not left else 'Modified managed files preserved: '+', '.join(left));return 2 if left else 0
    if a.verify:print('Installation verified' if i.verify() else 'Not installed or changed');return 0 if i.verify() else 1
    product=a.product
    if not product:
        for n,item in enumerate(i.manifest['products'],1):print(f'{n:2}. {item["name"]:18} '+('AVAILABLE — desktop preview' if item.get('available') else 'NOT AVAILABLE — planned'))
        value=input('Choose product number (0 cancels): ').strip()
        if value=='0':return 0
        if not value.isdigit() or not 1<=int(value)<=len(i.manifest['products']):raise ValueError('Invalid product number')
        product=i.manifest['products'][int(value)-1]['slug']
        i.payload(product)
        print('Install for this user; no sudo, no project code or network downloads. Bundle is an unsigned preview.')
        if input('Continue? [y/N] ').strip().lower()!='y':return 0
    launcher=i.install(product);print(f'Installed CodeCedar; menu entry and icons registered. Launch: {launcher}')
    if str(i.bin) not in os.environ.get('PATH','').split(os.pathsep):print(f'Command is not yet in PATH; use the menu or {launcher}. No shell profile was modified.')
    return 0
if __name__=='__main__':
    try:sys.exit(main())
    except (ValueError,OSError,KeyError,json.JSONDecodeError) as e:print(f'Installation refused: {e}',file=sys.stderr);sys.exit(1)
