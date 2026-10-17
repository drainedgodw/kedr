#!/usr/bin/env python3
"""Capture real native window states. On Linux, run inside xvfb-run."""
import argparse,hashlib,json,os,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--workspace',type=Path,default=Path('tests/fixtures/studio'));p.add_argument('--out',type=Path,default=Path('ui-captures'));a=p.parse_args()
a.out.mkdir(parents=True,exist_ok=True)
scenes=[('editor-dark','editor',False,'1440x900',True),('editor-light','editor',True,'1440x900',True),('compact','editor',False,'900x640',True),('welcome','welcome',False,'1200x800',False),('palette','palette',False,'1440x900',True),('commands','commands',False,'1440x900',True),('settings','settings',False,'1440x900',True),('search','search',False,'1440x900',True),('unsaved','unsaved',False,'1440x900',True),('trust','trust',False,'1440x900',True),('error','error',False,'1440x900',True)]
scenes += [("keymap","keymap",False,"1440x900",True),("goto","goto",False,"1440x900",True),("find","find",False,"1440x900",True),("compact-keymap","keymap",False,"900x640",True),("compact-find","find",False,"900x640",True)]
scenes += [("remap","remap",False,"1440x900",True),("remap-conflict","remap-conflict",False,"1440x900",True),("remap-record","remap-record",False,"1440x900",True),("compact-remap","remap",False,"900x640",True),("compact-remap-conflict","remap-conflict",False,"900x640",True),("light-keymap","keymap",True,"1440x900",True),("light-remap","remap",True,"1440x900",True)]
scenes += [("problems", "problems", False, "1440x900", True), ("compact-problems", "problems", False, "900x640", True), ("light-problems", "problems", True, "1440x900", True), ("task-output", "output", False, "900x640", True)]
files=[]
for name,state,light,size,project in scenes:
    png=a.out/(name+'.png');cmd=[str(a.binary.resolve())]
    if project:cmd.append(str(a.workspace.resolve()))
    cmd+=['--capture',str(png.resolve()),'--state',state,'--size',size]
    if light:cmd.append('--light')
    subprocess.run(cmd,timeout=30,check=True)
    raw=png.read_bytes();assert raw[:8]==b'\x89PNG\r\n\x1a\n' and len(raw)>4096
    files.append({'name':name,'state':state,'size':size,'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
(a.out/'captures.json').write_text(json.dumps({'binary_sha256':hashlib.sha256(a.binary.read_bytes()).hexdigest(),'scenes':files,'manual_visual_review':'required separately; generation is not review'},indent=2)+'\n')
