// Replace only the owned stroke-based date body; all geometry stays in bounds.
const r=storage.refinement,result={updated:0};
if(typeof r?.dateBadge!=='function')throw new Error('Load the isolated date-badge helper first.');
for(const p of penpotUtils.getPages()){
  await penpot.openPage(p.id);const page=penpot.currentPage;
  const dates=penpotUtils.findShapes(s=>s.getPluginData('lightcal-date-icon')==='calendar-badge-v3',page.root);
  const block=penpot.history.undoBlockBegin();
  try{for(const n of dates){
    const parent=n.parent,parts=parent.children.filter(s=>s.getPluginData('lightcal-date-icon-part')===n.id);
    if(!parts.some(s=>s.name==='Live date icon body'&&s.type==='rectangle'))continue;
    const o=JSON.parse(n.getPluginData('lightcal-date-icon-origin'));
    for(const s of parts)s.remove();
    r.dateBadge(parent,o.x,o.y,o.size,n.characters,o.mono,o.dark,n);result.updated++;
  }}finally{penpot.history.undoBlockFinish(block);}
}
return result;
