// Normalize finished pages, correct text contrast and connect the review flow.
const r=storage.refinement,pages=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).filter(p=>p.getPluginData('lightcal-refinement')===r.version);
r.targets={};
let colors=0;
for(const page of pages) {
  await penpot.openPage(page);
  for(const s of penpotUtils.findShapes(()=>true,page.root)) {
    if(['Current month underline','Current month badge','Current month badge label'].includes(s.name)){s.remove();continue;}
    if(s.type==='text'&&s.name==='Today date'){s.x-=s.parentX-20;s.resize(220,s.height);if(!s.parent.name.startsWith('Language')||s.parent.name.startsWith('Language · en'))s.characters='Saturday, 31 October';}
    const color=s.fills?.[0]?.fillColor?.toUpperCase();
    if(s.name.startsWith('Date ')&&color==='#E5484D'){s.fills=[{fillColor:'#D73840',fillOpacity:1}];colors++;}
    if(color==='#7479E5'){s.fills=[{fillColor:'#5E63D6',fillOpacity:1}];colors++;}
    if(s.type==='text'&&s.name.startsWith('Day numeral')&&color==='#7C7C87'){s.fills=[{fillColor:'#73737D',fillOpacity:1}];colors++;}
  }
  for(const b of page.root.children.filter(s=>s.type==='board')) {
    if(b.name.includes('Calendar · November · 31'))b.name=b.name.replace('November · 31','November · 12');
    const parts=b.name.split(' · '),platform=b.getPluginData('lightcal-platform')||parts[0],section=b.getPluginData('lightcal-section')||parts[1],state=b.getPluginData('lightcal-state')||parts[2],theme=b.name.endsWith('dark')?'dark':'light';
    if(!['Windows','macOS'].includes(platform))continue;
    let key;
    if(section==='Calendar')key=state==='November'?'next':b.name.includes(' · 30 · ')?'selected':'calendar';
    else if(section==='Empty day')key='empty';
    else if(['General','Appearance','Calendars','About'].includes(section))key=state==='default'?section:state==='conflict'?'shortcut':state;
    if(key)r.targets[platform+'/'+theme+'/'+key]=b;
  }
}
// Menu states are generated once here instead of duplicating the default page.
let links=0;const unresolved=[];
for(const page of pages) {
  await penpot.openPage(page);
  for(const s of penpotUtils.findShapes(s=>!!s.getPluginData('lightcal-target'),page.root)) {
    let key=s.getPluginData('lightcal-target'),dest=r.targets[key];
    if(!dest) {
      const parts=key.split('/'),other=parts[1]==='light'?'dark':'light';
      dest=r.targets[parts[0]+'/'+other+'/'+parts[2]];
      // The prototype opens the account-selection screen for account management.
      if(!dest&&parts[2]==='manage')dest=r.targets[parts[0]+'/'+parts[1]+'/Calendars'];
    }
    if(!dest){unresolved.push({page:page.name,name:s.name,target:key});continue;}
    for(const interaction of [...s.interactions])interaction.remove();
    s.addInteraction('click',{type:'navigate-to',destination:dest});links++;
  }
}
// Keep just three deliberate presentation entry points, rather than auto-flows.
for(const page of pages) {
  await penpot.openPage(page);
  for(const flow of [...page.flows])page.removeFlow(flow);
  const entry=page.root.children.find(b=>b.name==='Lightcal · Start here')||page.root.children.find(b=>b.getPluginData('lightcal-section')==='Appearance'&&b.getPluginData('lightcal-state')==='default'&&b.name.endsWith('light'));
  if(entry)page.createFlow(entry.name==='Lightcal · Start here'?'Start here':entry.name.startsWith('Windows')?'Windows settings':'macOS settings',entry);
}
await r.page('00 · Overview');
return {colorCorrections:colors,links,unresolved};
