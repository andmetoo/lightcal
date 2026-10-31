// Render vector design deliverables. Dependencies stay in ignored target/.
// Setup: bun add --cwd target/design-tools @resvg/resvg-js@2.6.2
import { createRequire } from 'node:module';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
const root=resolve(import.meta.dirname,'../..');
const require=createRequire(resolve(root,'target/design-tools/package.json'));
const { Resvg }=require('@resvg/resvg-js');
const output=resolve(root,'assets/icons/refined/png');
const review=resolve(root,'design/review');
await mkdir(output,{recursive:true});await mkdir(review,{recursive:true});
const master=await readFile(resolve(root,'assets/icons/refined/lightcal.svg'),'utf8');
const preserved=[];
for(const size of [16,24,32,48,64,128,256,512,1024]) {
  const source=size<=32?await readFile(resolve(root,`assets/icons/refined/lightcal-${size}.svg`),'utf8'):master;
  const path=resolve(output,`icon-${size}.png`);
  const before=await readFile(path).catch(()=>null);
  const png=new Resvg(source,{fitTo:{mode:'width',value:size}}).render().asPng();
  await writeFile(path,png);
  if([16,24].includes(size)&&before)preserved.push({size,unchanged:createHash('sha256').update(before).digest('hex')===createHash('sha256').update(png).digest('hex')});
}
const embed=s=>'data:image/svg+xml;base64,'+Buffer.from(s).toString('base64');
let svg='<svg xmlns="http://www.w3.org/2000/svg" width="1100" height="540" viewBox="0 0 1100 540"><rect width="1100" height="540" rx="20" fill="#F6F6F8"/><text x="36" y="48" font-family="Onest" font-size="22" font-weight="600" fill="#202025">Lightcal · final icon family</text>';
svg+=`<image href="${embed(master)}" x="30" y="88" width="240" height="240"/>`;
for(const [i,size]of [16,24,32,48,64].entries()) {
  const source=size<=32?await readFile(resolve(root,`assets/icons/refined/lightcal-${size}.svg`),'utf8'):master,x=300+i*150;
  for(const [j,bg]of ['#FFFFFF','#252529'].entries()) {
    const y=110+j*134;
    svg+=`<rect x="${x}" y="${y}" width="128" height="110" rx="12" fill="${bg}"/><image href="${embed(source)}" x="${x+(128-size)/2}" y="${y+(110-size)/2}" width="${size}" height="${size}"/>`;
  }
  svg+=`<text x="${x+64}" y="398" text-anchor="middle" font-family="Onest" font-size="14" fill="#666670">${size} px</text>`;
}
svg+='<text x="38" y="470" font-family="Onest" font-size="16" fill="#666670">Centered master · pixel-aligned small drawings · transparent PNG exports</text></svg>';
const fontFiles=['Regular','Medium','SemiBold','Bold'].map(style=>resolve(root,`design/fonts/onest/Onest-${style}.ttf`));
const image=new Resvg(svg,{font:{fontFiles,loadSystemFonts:false}}).render().asPng();
await writeFile(resolve(review,'icon-family.png'),image);
console.log(JSON.stringify({pngs:9,preserved,review:'design/review/icon-family.png'}));
