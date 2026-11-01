// Preserve existing boards and links; remove only the dated icon's paper/cap.
const r=storage.refinement,font=penpot.fonts.all.find(f=>f.name==='Onest');
if(!font)throw new Error('Exact Onest is required.');
const pages=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).filter(p=>p.getPluginData('lightcal-refinement')===r.version);
const result={converted:0,pages:[]};
for(const [index,page]of pages.entries()) {
  if(r.batch&&(index<r.batch.start||index>=r.batch.end))continue;
  const bodies=penpotUtils.findShapes(s=>s.name==='Live date icon body',page.root);
  const existing=penpotUtils.findShapes(s=>s.getPluginData('lightcal-date-icon')==='numerals-only-v1',page.root);
  const oldInk=existing.filter(s=>s.fills[0]?.fillColor.toUpperCase()==='#D73840');
  if(!bodies.length&&!oldInk.length)continue;
  await penpot.openPage(page);
  for(const s of oldInk)s.fills=[{fillColor:'#CF2835',fillOpacity:1}];
  for(const body of bodies) {
    const b=body.parent,children=[...b.children],size=body.width;
    const x=body.parentX,y=body.parentY;
    const cap=children.find(s=>s.name==='Calendar cap'&&Math.abs(s.parentX-x)<.1&&Math.abs(s.parentY-y)<.1);
    const numeral=children.find(s=>s.name==='Live date numeral'&&s.parentX>=x&&s.parentX<x+size&&s.parentY>=y&&s.parentY<y+size);
    if(!cap||!numeral)throw new Error('Incomplete dated icon: '+b.name);
    let dark=b.getPluginData('lightcal-theme')==='dark';
    for(const surface of children.filter(s=>/^Dark surface |^Light surface |^Taskbar (dark|light)$/.test(s.name))) {
      if(x>=surface.parentX&&x+size<=surface.parentX+surface.width&&y>=surface.parentY&&y+size<=surface.parentY+surface.height)
        dark=/^Dark surface |^Taskbar dark$/.test(surface.name);
    }
    const mono=cap.fills[0].fillColor.toUpperCase()==='#262626'||b.name.startsWith('Windows · Taskbar and tray');
    font.applyToText(numeral,font.variants.find(v=>v.fontWeight==='700'&&v.fontStyle==='normal'));
    numeral.fontSize=String(size*.74);numeral.fontWeight='700';numeral.resize(size,size);
    penpotUtils.setParentXY(numeral,x,y-size*.02);
    numeral.fills=[{fillColor:mono?(dark?'#EEEEEE':'#141416'):(dark?'#FF8A90':'#CF2835'),fillOpacity:1}];
    numeral.setPluginData('lightcal-date-icon','numerals-only-v1');
    numeral.setPluginData('lightcal-font-weight','700');numeral.setPluginData('lightcal-font-family','Onest');
    body.remove();cap.remove();result.converted++;
  }
  for(const s of penpotUtils.findShapes(s=>s.type==='text',page.root)) {
    if(s.name==='Live icon description')s.characters='Only the date. More space for readable numerals.\nThe numeral changes at local midnight.';
    if(s.name==='Icon system description')s.characters='A recognizable app mark. A clear date in the tray.';
    if(s.name==='Live date detail')s.characters='Only the date, beside the system clock.';
  }
  result.pages.push(page.name);
}
return result;
