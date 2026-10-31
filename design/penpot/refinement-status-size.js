// Only status artwork, its macOS selector and its size specimens are changed.
const r=storage.refinement;
if(typeof r?.dateBadge!=='function')throw new Error('Load refinement-tray-calendar-helper.js first.');
const pages=penpotUtils.getPages(),before=new Map(),result={resized:0,macMenuIcons:0,macSelectors:0,macSpecimens:0,unrelatedChanges:[]};
const shellNames=['Native menu bar specimen','Menu bar app name','Menu bar menus','Selected Lightcal status item','Lightcal menu label','System status menu','Native macOS clock','Menu bar caption','Menu bar detail','macOS presentation description'];
const selectorNames=['Menu bar label','Date only option label','Weekday and date option','Menu label selection option 0','Menu label selection option 1'];
const specimenNames=['Date label sample','Weekday date label sample','Date label','Weekday date label','Menu label guidance'];
const owned=s=>{
  for(let q=s;q;q=q.parent){
    if(q.getPluginData('lightcal-date-icon')||q.getPluginData('lightcal-date-icon-part'))return true;
    if(q.parent?.name.startsWith('macOS · Menu bar and Dock · ')&&shellNames.includes(q.name))return true;
    if(q.parent?.name.startsWith('macOS · App · ')&&selectorNames.includes(q.name))return true;
    if(q.parent?.name.startsWith('Icon system · ')&&specimenNames.includes(q.name))return true;
  }return false;
};
const describe=s=>JSON.stringify([s.name,s.type,s.parent?.id,s.parentX,s.parentY,s.width,s.height,s.fills,s.strokes,s.type==='text'?[s.characters,s.fontFamily,s.fontSize,s.fontWeight]:null]);
for(const p of pages)for(const s of penpotUtils.findShapes(()=>true,penpotUtils.getPageById(p.id).root))if(!owned(s))before.set(s.id,describe(s));
for(const p of pages){
  await penpot.openPage(p.id);const page=penpot.currentPage,changedText=[];
  const block=penpot.history.undoBlockBegin();
  try{
    const dates=penpotUtils.findShapes(s=>s.getPluginData('lightcal-date-icon')==='calendar-badge-v2',page.root);
    for(const n of dates){
      const o=JSON.parse(n.getPluginData('lightcal-date-icon-origin')),parent=n.parent;
      for(const part of [...parent.children].filter(s=>s.getPluginData('lightcal-date-icon-part')===n.id))part.remove();
      r.dateBadge(parent,o.x,o.y,o.size,n.characters,o.mono,o.dark,n);changedText.push(n);result.resized++;
    }
    for(const b of page.root.children.filter(s=>/^macOS · Menu bar and Dock · /.test(s.name)&&s.getPluginData('lightcal-status-size')!=='v3')){
      const get=name=>b.children.filter(s=>s.name===name);
      for(const bar of get('Native menu bar specimen')){
        const yy=bar.parentY,dark=bar.fills[0].fillColor.toUpperCase()==='#222222';bar.resize(bar.width,24);bar.borderRadius=4;
        const at=(name)=>get(name).find(s=>s.parentY>=yy&&s.parentY<yy+36);
        for(const name of ['Menu bar app name','Menu bar menus','Native macOS clock']){const s=at(name);s.resize(s.width,24);changedText.push(s);}
        const slot=at('Selected Lightcal status item');slot.resize(24,24);slot.borderRadius=4;penpotUtils.setParentXY(slot,665,yy);
        slot.setPluginData('lightcal-status-length','NSStatusItem.squareLength');
        const n=at('Lightcal menu label');r.dateBadge(b,668,yy+3,18,'31',false,dark,n);changedText.push(n);result.macMenuIcons++;
        const system=at('System status menu');penpotUtils.setParentXY(system,system.parentX,yy+4);
      }
      for(const s of get('macOS presentation description')){s.characters='The same calendar identity, sized for the native menu bar.';changedText.push(s);}
      for(const s of get('Menu bar caption')){s.characters='Shared calendar identity';changedText.push(s);}
      for(const s of get('Menu bar detail')){s.characters='18 pt artwork. System-sized button. Color or monochrome.';changedText.push(s);}
      b.setPluginData('lightcal-status-size','v3');
    }
    for(const b of page.root.children.filter(s=>s.name.startsWith('macOS · App · '))){
      const label=b.children.find(s=>s.name==='Menu bar label');if(!label)continue;
      const n0=b.children.find(s=>s.name==='Date only option label'),n1=b.children.find(s=>s.name==='Weekday and date option');
      if(!n0||!n1)throw new Error('Incomplete menu-bar selector: '+b.name);
      label.characters='Menu bar icon';label.name='Menu bar icon';changedText.push(label);
      const selected=b.children.find(s=>s.name==='Menu label selection option 1'),unselected=b.children.find(s=>s.name==='Menu label selection option 0');
      const paints={fills:unselected.fills,strokes:unselected.strokes};unselected.fills=selected.fills;unselected.strokes=selected.strokes;selected.fills=paints.fills;selected.strokes=paints.strokes;
      const dark=b.getPluginData('lightcal-theme')==='dark';
      for(const [i,n]of [n0,n1].entries()){
        const x=268+i*76;n.characters=i?'Mono':'Color';n.name=i?'Monochrome label':'Color label';
        n.resize(42,28);penpotUtils.setParentXY(n,x+27,222);n.fontSize='10.5';n.fontWeight=i?'400':'600';
        n.setPluginData('lightcal-font-weight',i?'400':'600');changedText.push(n);
        changedText.push(r.dateBadge(b,x+6,228,16,'31',!!i,dark));
      }result.macSelectors++;
    }
    for(const b of page.root.children.filter(s=>s.name.startsWith('Icon system · ')&&s.getPluginData('lightcal-mac-status-size')!=='v3')){
      const dark=b.getPluginData('lightcal-theme')==='dark';
      for(const [i,names]of [['Date label sample','Date label'],['Weekday date label sample','Weekday date label']].entries()){
        const frame=b.children.find(s=>s.name===names[0]),text=b.children.find(s=>s.name===names[1]),x=i?872:744;
        frame.resize(24,24);frame.borderRadius=4;
        text.characters=i?'Mono':'Color';text.resize(i?80:72,24);text.fontSize='12';text.fontWeight='500';text.setPluginData('lightcal-font-weight','500');
        penpotUtils.setParentXY(text,i?908:780,632);text.align='left';changedText.push(text);
        changedText.push(r.dateBadge(b,x+3,635,18,'31',!!i,dark));result.macSpecimens++;
      }
      const guide=b.children.find(s=>s.name==='Menu label guidance');guide.characters='Same calendar silhouette. 18 pt artwork in a system-sized status item.';changedText.push(guide);
      b.setPluginData('lightcal-mac-status-size','v3');
    }
    for(const s of changedText){s.getRange(0,s.characters.length).fills=s.fills;const size=s.fontSize;s.setPluginData('lightcal-status-refresh-size',size);s.fontSize=String(Number(size)+.01);}
    if(changedText.length)await new Promise(resolve=>setTimeout(resolve,250));
    for(const s of changedText)s.fontSize=s.getPluginData('lightcal-status-refresh-size');
    if(changedText.length)await new Promise(resolve=>setTimeout(resolve,250));
  }finally{penpot.history.undoBlockFinish(block);}
}
const after=new Map();for(const p of pages)for(const s of penpotUtils.findShapes(()=>true,penpotUtils.getPageById(p.id).root))after.set(s.id,describe(s));
for(const [id,state]of before)if(after.get(id)!==state)result.unrelatedChanges.push(id);
if(result.unrelatedChanges.length)throw new Error(JSON.stringify(result));
return result;
