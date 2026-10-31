// Migrate the one existing macOS shortcut-conflict specimen without rebuilding.
const r=storage.refinement;
const pages=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).filter(p=>p.getPluginData('lightcal-refinement')===r.version);
const fixed=[];
for(const page of pages) {
  const labels=penpotUtils.findShapes(s=>s.type==='text'&&s.characters==='⌥ ⇧ C',page.root);
  if(!labels.length)continue;
  await penpot.openPage(page);
  for(const label of labels) {
    const b=label.parent,t=r[b.getPluginData('lightcal-theme')||'light'];
    const control=b.children.find(s=>s.name==='Record shortcut'&&s.type==='rectangle');
    if(!control)throw new Error('Shortcut button is missing');
    r.shortcutSymbols(b,'Record shortcut',control.parentX,control.parentY,control.width,t,label);
    fixed.push({page:page.name,board:b.name,id:b.id});
  }
}
return {fixed};
