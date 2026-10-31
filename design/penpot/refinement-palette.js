// Gray + Teal, owner-requested removal of violet from every active specimen.
// Batch indices address the stable top-level board list, not page indices.
const r=storage.refinement,palette=r.palette;
if(!palette)throw new Error('Load gray-teal.json into storage.refinement.palette first.');
const pages=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).filter(p=>p.getPluginData('lightcal-refinement')===r.version);
const boards=pages.flatMap(page=>page.root.children.filter(s=>s.type==='board').map(board=>({page,board})));
const result={boards:0,paints:0,markers:0};
let activePage;
const color=value=>palette.replacements[value?.toUpperCase()]||value;
function paints(s) {
  if(s.fills?.some(f=>color(f.fillColor)!==f.fillColor)) {
    s.fills=s.fills.map(f=>({...f,...(f.fillColor?{fillColor:color(f.fillColor)}:{})}));result.paints++;
  }
  if(s.strokes?.some(f=>color(f.strokeColor)!==f.strokeColor)) {
    s.strokes=s.strokes.map(f=>({...f,...(f.strokeColor?{strokeColor:color(f.strokeColor)}:{})}));result.paints++;
  }
  if(s.type==='text'&&palette.replacements[s.characters?.toUpperCase()])s.characters=color(s.characters);
}
for(const [index,{page,board}]of boards.entries()) {
  if(r.batch&&(index<r.batch.start||index>=r.batch.end))continue;
  if(activePage!==page.id){await penpot.openPage(page);activePage=page.id;}
  const shapes=[board,...penpotUtils.findShapes(()=>true,board)].filter((s,i,a)=>a.findIndex(x=>x.id===s.id)===i);
  shapes.forEach(paints);
  for(const b of shapes.filter(s=>s.type==='board')) {
    const t=palette[b.getPluginData('lightcal-theme')||'light'];
    for(const s of b.children) {
      if(s.type==='rectangle'&&s.fills[0]?.fillColor.toUpperCase()===t.selected&&s.strokes?.length)
        s.strokes=s.strokes.map(stroke=>({...stroke,strokeColor:t.selectedBorder}));
      if(s.type==='text'&&/^Shade weekday label/.test(s.name)&&s.fontWeight==='600')
        s.fills=[{fillColor:t.ink,fillOpacity:1}];
    }
    const tab=b.children.find(s=>s.name.startsWith('Navigate to ')&&s.fills[0]?.fillColor.toUpperCase()===t.selected);
    if(tab) {
      const section=tab.name.replace('Navigate to ',''),label=b.children.find(s=>s.name===section+' navigation label');
      const visible={Appearance:'Calendar',Calendars:'Accounts',General:'App'}[section];
      const icon=b.children.find(s=>s.name===visible+' navigation icon');
      if(label)label.fills=[{fillColor:t.ink,fillOpacity:1}];
      if(icon)for(const part of [icon,...penpotUtils.findShapes(()=>true,icon)]) {
        if(part.strokes?.length)part.strokes=part.strokes.map(stroke=>({...stroke,strokeColor:t.ink}));
      }
      let marker=b.children.find(s=>s.name==='Active tab marker');
      if(!marker){marker=r.rect(b,'Active tab marker',tab.parentX+56,tab.parentY+29,24,2,t.accent,1);result.markers++;}
      else marker.fills=[{fillColor:t.accent,fillOpacity:1}];
    }
  }
  board.setPluginData('lightcal-palette','Gray + Teal; Radix-derived; 2026-10-31');
  result.boards++;
}
return result;
