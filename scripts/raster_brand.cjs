const fs=require('fs'),path=require('path'),sharp=require('sharp');
const root=path.resolve(__dirname,'..');
(async()=>{for(const name of fs.readdirSync(path.join(root,'assets/brand')).filter(x=>x.endsWith('.svg'))){for(const size of [48,128,256,512]){await sharp(path.join(root,'assets/brand',name)).resize(size,size).png().toFile(path.join(root,'assets/icons',`${name.slice(0,-4)}-${size}.png`));}}})().catch(e=>{console.error(e);process.exit(1)});
