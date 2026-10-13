#!/usr/bin/env python3
"""Generate editable vector identities, product PNGs and Windows icon containers."""
import colorsys,html,json,subprocess
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
root=Path(__file__).resolve().parents[1]
font=TTFont(subprocess.check_output(['fc-match','Arial:style=Bold','-f','%{file}']).decode()); glyphs=font.getGlyphSet(); cmap=font.getBestCmap(); units=font['head'].unitsPerEm
products=json.loads((root/'products/catalog.json').read_text())['products']
entries=[dict(slug='kedr',name='Kedr',monogram='K',kind='Family',status='brand')]+products
frames=[]
for i,p in enumerate(entries):
    rgb=colorsys.hsv_to_rgb((0.56+i*0.381966)%1,.65,.88)
    accent='#'+''.join(f'{round(c*255):02X}' for c in rgb)
    size=34; scale=size/units; total=sum(font['hmtx'][cmap[ord(c)]][0]*scale for c in p['monogram']);x=(128-total)/2
    paths=[]
    for c in p['monogram']:
        pen=SVGPathPen(glyphs);trans=TransformPen(pen,(scale,0,0,-scale,x,108));glyphs[cmap[ord(c)]].draw(trans);paths.append(f'<path d="{pen.getCommands()}"/>');x+=font['hmtx'][cmap[ord(c)]][0]*scale
    svg=f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" role="img" aria-labelledby="title"><title id="title">{p['name']} logo</title><rect x="4" y="4" width="120" height="120" rx="24" fill="#102033"/><path d="M104 4H100L124 28V48L80 4Z" fill="{accent}"/><path d="M124 90V100L100 124H90Z" fill="{accent}"/><path d="M64 18 42 40H56L34 62H56V72H64V54H44L64 34Z" fill="{accent}"/><path d="M67 23 87 43H73L96 66H72V76H67V58H86L67 39Z" fill="#FFFFFF"/><g fill="#FFFFFF">{''.join(paths)}</g></svg>'''
    (root/f'assets/brand/{p["slug"]}.svg').write_text(svg+'\n')
    frames.append(f'<section class="logo-frame" id="{p["slug"]}">{svg}<h2>{html.escape(p["name"])}</h2><p>{p["kind"]} · {p["status"]}</p></section>')
css='''body{margin:0;font:16px/1.5 Arial,sans-serif;background:#f6f8fa;color:#263344}main{max-width:1200px;margin:auto;display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));padding:24px;gap:24px}.logo-frame{box-sizing:border-box;min-width:0;height:320px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:16px;background:#fff;border:1px solid #dce2e9;border-radius:12px}.logo-frame svg{width:128px;height:128px}h2{font-size:24px;margin:0}p{font-size:14px;margin:0;color:#59697d;text-align:center}@media(max-width:500px){main{grid-template-columns:1fr;padding:16px}}@media(prefers-color-scheme:dark){body{background:#151b24;color:#e0e6ef}.logo-frame{background:#1b2430;border-color:#3d4857}p{color:#acb8c8}}'''
(root/'assets/brand/catalog.html').write_text('<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Kedr identities</title><style>'+css+'</style><main>'+''.join(frames)+'</main></html>')
(root/'assets/brand/README.md').write_text('''# Kedr identity, revision 2

Original split-cedar mark on a navy field, larger white outlined monograms and
per-product accent colors. Color is not the sole identifier; every mark has a
unique monogram and accessible SVG title. These are not JetBrains marks.

SVGs are editable and need no external fonts at runtime. PNGs at 48/128/256/512
and Windows ICOs are generated under assets/icons by scripts/generate_brand.py.
The generator needs fontTools, Arial Bold, Node sharp and Pillow; runtime apps do
not need those design dependencies. catalog.html is only an identity preview.

Naming/logo files do not make planned products installable.
''')
subprocess.run(['node',str(root/'scripts/raster_brand.cjs')],check=True)
from PIL import Image
for p in entries:
    image=Image.open(root/f'assets/icons/{p["slug"]}-512.png');image.save(root/f'assets/icons/{p["slug"]}.ico',sizes=[(16,16),(32,32),(48,48),(64,64),(128,128),(256,256)])
print('Generated',len(entries),'identities and raster/icon variants.')
