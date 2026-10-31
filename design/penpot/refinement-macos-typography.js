// Owner-selected native macOS typography. Upload Apple's design faces first.
// Save a permanent version; run page batches 2:3 and 3:4 after loading core.
const r=storage.refinement;
for(const family of ['SF Pro Text','SF Pro Display']) {
  const font=penpot.fonts.all.find(f=>f.name===family);
  if(!font)throw new Error(`Upload ${family} in the Penpot team's Fonts section first.`);
  for(const weight of ['400','500','600','700'])if(!font.variants.some(v=>v.fontWeight===weight&&v.fontStyle==='normal'))
    throw new Error(`Missing ${family} ${weight} normal`);
}
const pages=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).filter(p=>p.getPluginData('lightcal-refinement')===r.version);
const result={boards:0,fonts:{},pages:[]};
for(const [index,page]of pages.entries()) {
  if(r.batch&&(index<r.batch.start||index>=r.batch.end))continue;
  const boards=page.root.children.filter(s=>s.type==='board'&&(s.getPluginData('lightcal-platform')==='macOS'||s.name.startsWith('macOS · ')));
  if(!boards.length)continue;
  await penpot.openPage(page);
  for(const board of boards) {
    for(const s of penpotUtils.findShapes(s=>s.type==='text',board)) {
      const weight=s.getPluginData('lightcal-font-weight')||s.fontWeight||'400',size=s.fontSize;
      const family=r.fontFamily(s.parent,s.characters,size);
      const font=penpot.fonts.all.find(f=>f.name===family);
      const variant=font?.variants.find(v=>v.fontWeight===weight&&v.fontStyle==='normal');
      if(!variant)throw new Error(`Missing ${family} ${weight}`);
      font.applyToText(s,variant);s.fontSize=size;s.fontWeight=weight;
      s.setPluginData('lightcal-font-weight',weight);s.setPluginData('lightcal-font-family',family);
      result.fonts[family]=(result.fonts[family]||0)+1;
    }
    board.setPluginData('lightcal-typography','macOS system: SF Pro Text <20px, SF Pro Display >=20px');
    result.boards++;
  }
  result.pages.push(page.name);
}
return result;
