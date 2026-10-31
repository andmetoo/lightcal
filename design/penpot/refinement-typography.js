// Owner-selected platform fonts. Preserve geometry, IDs, weights and links.
// Save a permanent version first; run in one-page batches 0:1 through 6:7.
const r=storage.refinement;
const primary=penpot.fonts.all.find(f=>f.name==='Onest');
const japanese=penpot.fonts.all.find(f=>f.name==='Noto Sans JP');
if(!primary||!japanese)throw new Error('Exact Onest and Noto Sans JP are required.');
const pages=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).filter(p=>p.getPluginData('lightcal-refinement')===r.version);
const result={pages:[],fonts:{},weights:{}};
for(const [index,page]of pages.entries()) {
  if(r.batch&&(index<r.batch.start||index>=r.batch.end))continue;
  await penpot.openPage(page);
  for(const s of penpotUtils.findShapes(s=>s.type==='text',page.root)) {
    const weight=s.getPluginData('lightcal-font-weight')||s.fontWeight||'400';
    const size=s.fontSize;
    if(s.name.endsWith(' typography guidance'))s.characters=s.characters.replace(/^Inter /,'Onest ');
    const family=r.fontFamily(s.parent,s.characters,size);
    const font=penpot.fonts.all.find(f=>f.name===family);
    if(!font)throw new Error('Exact font family is required: '+family);
    const variant=font.variants.find(v=>v.fontWeight===weight&&v.fontStyle==='normal');
    if(!variant)throw new Error(`Missing ${font.name} weight ${weight}`);
    font.applyToText(s,variant);s.fontSize=size;s.fontWeight=weight;
    s.setPluginData('lightcal-font-weight',weight);
    s.setPluginData('lightcal-font-family',font.name);
    result.fonts[font.name]=(result.fonts[font.name]||0)+1;
    result.weights[weight]=(result.weights[weight]||0)+1;
  }
  page.setPluginData('lightcal-typography','Windows Onest / macOS SF Pro / Japanese Noto Sans JP');
  result.pages.push(page.name);
}
return result;
