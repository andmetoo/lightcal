// Read-only input for native font coverage checks. Store output in target/.
const pages=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).filter(p=>p.getPluginData('lightcal-refinement')==='2026-10-31');
const texts=[];
for(const page of pages)for(const s of penpotUtils.findShapes(s=>s.type==='text',page.root))
  texts.push({id:s.id,page:page.name,name:s.name,text:s.characters,font:s.fontFamily,weight:s.fontWeight,size:s.fontSize,width:s.width,height:s.height});
return texts;
