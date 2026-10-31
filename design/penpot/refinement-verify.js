// Read-only geometry and text-box QA; pixel inspection is a separate gate.
const pages=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).filter(p=>p.getPluginData('lightcal-refinement')==='2026-10-31');
const violations=[],duplicates=[],inventory=[];
const typography={fonts:{},weights:{},violations:[]};
for(const page of pages) {
  for(const text of penpotUtils.findShapes(s=>s.type==='text',page.root)) {
    const expected=storage.refinement.fontFamily(text.parent,text.characters,text.fontSize);
    const family=text.fontFamily,weight=text.fontWeight;
    typography.fonts[family]=(typography.fonts[family]||0)+1;
    typography.weights[weight]=(typography.weights[weight]||0)+1;
    if(family!==expected||weight!==text.getPluginData('lightcal-font-weight'))
      typography.violations.push({page:page.name,name:text.name,family,expected,weight});
  }
  const names=new Set();
  for(const board of page.root.children.filter(s=>s.type==='board')) {
    if(names.has(board.name))duplicates.push({page:page.name,name:board.name});names.add(board.name);
    const children=penpotUtils.analyzeDescendants(board,(root,s)=>{
      const a=s.bounds,b=root.bounds,eps=1;
      if(a.x<b.x-eps||a.y<b.y-eps||a.x+a.width>b.x+b.width+eps||a.y+a.height>b.y+b.height+eps)return {name:s.name,type:s.type,x:s.parentX,y:s.parentY,width:s.width,height:s.height};
    });
    for(const item of children)violations.push({page:page.name,board:board.name,...item.result});
    inventory.push({page:page.name,pageId:page.id,id:board.id,name:board.name,width:Math.round(board.width),height:Math.round(board.height)});
  }
}
return {pages:pages.length,boards:inventory.length,duplicates,violations,typography,inventory};
