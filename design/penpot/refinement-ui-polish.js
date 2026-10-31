// Apply the UI review to existing owned boards without rebuilding their flows.
// Idempotent; take a permanent version first. Process one or two pages per call.
const r=storage.refinement;
const primary=penpot.fonts.all.find(f=>f.name==='Onest');
const japanese=penpot.fonts.all.find(f=>f.name==='Noto Sans JP');
if(!primary||!japanese)throw new Error('Exact Onest and Noto Sans JP are required.');
const pages=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).filter(p=>p.getPluginData('lightcal-refinement')===r.version);
const counts={fonts:0,controls:0,segments:0,icons:0,focus:0,dates:0};
function placeBefore(parent,newShapes,before) {
  const index=parent.children.findIndex(s=>s.id===before.id);
  newShapes.forEach((s,i)=>s.setParentIndex(index+i));
}
function frame(b,name,x,y,w,count,t,selected,oldNames,before) {
  for(const s of [...b.children])if(oldNames.includes(s.name)||s.name===name+' track'||s.name.startsWith(name+' option '))s.remove();
  const result=r.segmentFrame(b,name,x,y,w,count,t,selected);
  placeBefore(b,[result.track,...result.slots.map(s=>s.hit)],before);
  counts.segments++;
}
for(const [index,page]of pages.entries()) {
  if(r.batch&&(index<r.batch.start||index>=r.batch.end))continue;
  await penpot.openPage(page);
  for(const b of penpotUtils.findShapes(s=>s.type==='board',page.root)) {
    const t=r[b.getPluginData('lightcal-theme')||'light'];
    let children=[...b.children];
    for(const body of children.filter(s=>s.name==='App icon body'&&!s.getPluginData('lightcal-icon-optics'))) {
      const size=body.width,start=children.findIndex(s=>s.id===body.id);
      const tile=size*.19,gap=size*.12,cap=Math.round(size*.27);
      const delta=size>32?cap+(size-cap-(tile*2+gap))/2-size*.42:size>24?-1:0;
      for(const s of children.slice(start+1,start+7))if(s.name==='Calendar day tile')s.y+=delta;
      body.setPluginData('lightcal-icon-optics','centered-v1');counts.icons++;
    }
    const themeLabel=children.find(s=>s.name==='System theme label');
    if(themeLabel)frame(b,'Theme selection',216,102,204,3,t,0,['System theme option','Light theme option','Dark theme option'],themeLabel);
    children=[...b.children];
    const trayLabel=children.find(s=>s.name==='Color label');
    if(trayLabel) {
      const already=children.some(s=>s.name==='Tray palette selection track');
      if(!already)for(const s of children) {
        if(['Monochrome label','Live date numeral','Live date icon body','Calendar cap'].includes(s.name)&&s.parentY>=222&&s.parentY<=254&&s.parentX>=344)s.x-=4;
      }
      const anchor=b.children.find(s=>s.name==='Live date icon body'&&s.parentX>=268&&s.parentY>=222&&s.parentY<=254)||trayLabel;
      frame(b,'Tray palette selection',268,222,152,2,t,0,['Color icon selected','Monochrome icon option'],anchor);
      trayLabel.fontWeight='600';
      const mono=b.children.find(s=>s.name==='Monochrome label');if(mono)mono.fontWeight='400';
    }
    const menuLabel=b.children.find(s=>s.name==='Weekday and date option');
    if(menuLabel) {
      const anchor=b.children.find(s=>s.name==='Date only option label')||menuLabel;
      frame(b,'Menu label selection',268,222,152,2,t,1,['Date only option','Selected menu label'],anchor);
      penpotUtils.setParentXY(menuLabel,344,222);menuLabel.resize(76,28);menuLabel.fontSize='11';
      const date=b.children.find(s=>s.name==='Date only option label');
      if(date){penpotUtils.setParentXY(date,268,222);date.resize(76,28);date.fontSize='11';}
    }
    children=[...b.children];
    for(const control of children.filter(s=>s.type==='rectangle'&&Math.abs(s.height-32)<.01)) {
      const label=children.find(s=>s.type==='text'&&s.name===control.name+' label');
      const value=children.find(s=>s.type==='text'&&s.name===control.name+' value');
      if(!label&&!value)continue;
      control.resize(control.width,28);control.borderRadius=7;
      const text=label||value;text.resize(text.width,28);
      const arrow=children.find(s=>s.name===control.name+' chevron');if(arrow)arrow.y-=2;
      counts.controls++;
    }
    for(const s of [...b.children]) {
      if(s.name==='Today and keyboard selection'){s.remove();counts.focus++;}
      if(s.type==='text'&&s.name==='Today date') {
        const chars=b.name.startsWith('Language')&&!b.name.startsWith('Language · en')?s.characters:'Saturday, 31 October';
        const at=b.children.findIndex(c=>c.id===s.id),x=s.parentX,y=s.parentY,w=s.width,h=s.height,color=s.fills[0].fillColor;
        s.remove();const fresh=r.text(b,'Today date',chars,x,y,w,h,11.5,400,color);fresh.setParentIndex(at);counts.dates++;
      }
    }
  }
  for(const s of penpotUtils.findShapes(s=>s.type==='text',page.root)) {
    const weight=s.getPluginData('lightcal-font-weight')||s.fontWeight||'400',size=s.fontSize;
    const family=r.fontFamily(s.parent,s.characters,size);
    const font=penpot.fonts.all.find(f=>f.name===family);
    if(!font)throw new Error('Exact font family is required: '+family);
    const variant=font.variants.find(v=>v.fontWeight===weight&&v.fontStyle!=='italic');
    font.applyToText(s,variant);s.fontSize=size;s.fontWeight=weight;s.setPluginData('lightcal-font-weight',weight);counts.fonts++;
  }
}
return counts;
