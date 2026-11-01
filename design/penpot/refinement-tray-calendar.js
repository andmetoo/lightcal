// Update only live Windows date artwork. Preserve the owner's other edits.
const r=storage.refinement,result={updated:0,pages:[],unrelatedChanges:[]};
if(typeof r?.dateBadge!=='function')throw new Error('Load refinement-tray-calendar-helper.js first.');
const pages=penpotUtils.getPages();
const ignored=s=>s.getPluginData('lightcal-date-icon')||s.getPluginData('lightcal-date-icon-part')||['Live icon description','Live date detail'].includes(s.name);
const describe=s=>JSON.stringify([s.name,s.type,s.parent?.id,s.parentX,s.parentY,s.width,s.height,s.fills,s.strokes,s.type==='text'?[s.characters,s.fontFamily,s.fontSize,s.fontWeight]:null]);
const before=new Map();
for(const p of pages)for(const s of penpotUtils.findShapes(()=>true,penpotUtils.getPageById(p.id).root))if(!ignored(s))before.set(s.id,describe(s));
for(const p of pages){
  await penpot.openPage(p.id);
  const page=penpot.currentPage;
  const numerals=penpotUtils.findShapes(s=>s.getPluginData('lightcal-date-icon')==='numerals-only-v1',page.root);
  const finished=penpotUtils.findShapes(s=>s.getPluginData('lightcal-date-icon')==='calendar-badge-v2',page.root);
  if(!numerals.length&&!finished.length)continue;
  const block=penpot.history.undoBlockBegin();
  try{for(const numeral of numerals){
    const parent=numeral.parent,size=numeral.width,x=numeral.parentX,y=numeral.parentY+size*.02;
    let board=parent;while(board.parent?.type==='board')board=board.parent;
    let dark=board.getPluginData('lightcal-theme')==='dark';
    for(const s of parent.children.filter(s=>/^Dark surface |^Light surface |^Taskbar (dark|light)$/.test(s.name)))
      if(x>=s.parentX&&x+size<=s.parentX+s.width&&y>=s.parentY&&y+size<=s.parentY+s.height)dark=/^Dark surface |^Taskbar dark$/.test(s.name);
    const ink=numeral.fills[0]?.fillColor?.toUpperCase();
    const mono=['#141416','#EEEEEE','#141414','#F4F4F6'].includes(ink);
    r.dateBadge(parent,x,y,size,numeral.characters,mono,dark,numeral);result.updated++;
  }
  for(const s of penpotUtils.findShapes(s=>s.type==='text',page.root)){
    if(s.name==='Live icon description')s.characters='A compact calendar page with a clear date.\nA slim header keeps its identity at small sizes.';
    if(s.name==='Live date detail')s.characters='A recognizable calendar, beside the system clock.';
  }
  // Penpot may retain inline text paint/layout after setting Text.fills.
  // Reapply the actual range and let the renderer recalculate before export.
  const dates=penpotUtils.findShapes(s=>s.getPluginData('lightcal-date-icon')==='calendar-badge-v2'||['Live icon description','Live date detail'].includes(s.name),page.root);
  const sizes=dates.map(s=>({s,size:s.fontSize}));
  for(const {s,size}of sizes){s.getRange(0,s.characters.length).fills=s.fills;s.fontSize=String(Number(size)+.01);}
  await new Promise(resolve=>setTimeout(resolve,250));
  for(const {s,size}of sizes)s.fontSize=size;
  await new Promise(resolve=>setTimeout(resolve,250));
  }finally{penpot.history.undoBlockFinish(block);}
  result.pages.push({name:p.name,icons:numerals.length});
}
for(const p of pages)for(const s of penpotUtils.findShapes(()=>true,penpotUtils.getPageById(p.id).root))if(before.has(s.id)&&before.get(s.id)!==describe(s))result.unrelatedChanges.push({id:s.id,name:s.name});
if(result.unrelatedChanges.length)throw new Error(JSON.stringify(result.unrelatedChanges));
return result;
