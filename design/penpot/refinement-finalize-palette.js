// Refresh actual rendered paints/text, not just the values returned by getters.
// Batch indices are top-level board indices. All temporary changes are restored.
const r=storage.refinement;
const entries=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).filter(p=>p.getPluginData('lightcal-refinement')===r.version).flatMap(p=>p.root.children.filter(b=>b.type==='board').map(b=>({pageId:p.id,boardId:b.id})));
const result={boards:0,texts:0,badges:0};
const nudge=value=>{
  if(!/^#[0-9a-f]{6}$/i.test(value||''))return value;
  return value.slice(0,5)+(parseInt(value.slice(5),16)^1).toString(16).padStart(2,'0');
};
for(const [index,item]of entries.entries()) {
  if(r.batch&&(index<r.batch.start||index>=r.batch.end))continue;
  await penpot.openPage(item.pageId);
  const board=penpot.currentPage.root.children.find(b=>b.id===item.boardId);
  const shapes=[board,...penpotUtils.findShapes(()=>true,board)].filter((s,i,a)=>a.findIndex(x=>x.id===s.id)===i);
  for(const b of shapes.filter(s=>s.type==='board')) {
    for(const badge of b.children.filter(s=>s.type==='rectangle'&&(s.name==='Account state'||/(read-only|reconnect) status$/.test(s.name)))) {
      const dark=badge.name.startsWith('dark ')||b.getPluginData('lightcal-theme')==='dark';
      badge.fills=[{fillColor:dark?'#313131':'#E8E8E8',fillOpacity:1}];
      const label=b.children.find(s=>s.type==='text'&&s.name===badge.name+' label');
      if(label)label.fills=[{fillColor:dark?'#EEEEEE':'#202020',fillOpacity:1}];
      result.badges++;
    }
  }
  const block=penpot.history.undoBlockBegin(),saved=[];
  try {
    for(const s of shapes) {
      const fills=Array.isArray(s.fills)?s.fills.map(f=>({...f})):null;
      const strokes=s.strokes?.map(f=>({...f}));
      const size=s.type==='text'?s.fontSize:null;
      saved.push({s,fills,strokes,size});
      if(s.type==='text'&&s.characters.length&&fills) {
        s.getRange(0,s.characters.length).fills=fills;
        if(Number.isFinite(Number(size)))s.fontSize=String(Number(size)+0.01);
        result.texts++;
      } else if(fills?.length)s.fills=fills.map(f=>({...f,...(f.fillColor?{fillColor:nudge(f.fillColor)}:{})}));
      if(strokes?.length)s.strokes=strokes.map(f=>({...f,...(f.strokeColor?{strokeColor:nudge(f.strokeColor)}:{})}));
    }
    await new Promise(resolve=>setTimeout(resolve,180));
  } finally {
    if(penpot.currentPage.id!==item.pageId)await penpot.openPage(item.pageId);
    for(const {s,fills,strokes,size}of saved) {
      if(fills)s.fills=fills;
      if(strokes)s.strokes=strokes;
      if(size!==null)s.fontSize=size;
    }
    penpot.history.undoBlockFinish(block);
  }
  await new Promise(resolve=>setTimeout(resolve,180));
  result.boards++;
}
return result;
