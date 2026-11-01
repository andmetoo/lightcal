// Lightcal refinement, 2026-10-31. Load once, then run the page builders.
// The original seven pages are preserved; these builders only own v2 pages.
if (penpot.currentFile?.id !== 'c2d27051-3cd1-813e-8008-dd9b45707753') {
  throw new Error('Connect the Lightcal Penpot file before running this script.');
}
const prior=storage.refinement;
storage.refinement = {
  version: '2026-10-31',
  boards: prior?.boards||[],
  targets: prior?.targets||{},
  settingsContent: prior?.settingsContent,
  light: { canvas:'#FFFFFF', panel:'#F9F9F9', group:'#F0F0F0', line:'#D9D9D9', border:'#BBBBBB', ink:'#202020', secondary:'#646464', muted:'#646464', outside:'#6E6E6E', weekend:'#F0F0F0', accent:'#00796B', soft:'#E0E0E0', selectedBorder:'#838383', coral:'#E5484D', success:'#28774B', successSoft:'#EDF7F0', danger:'#BC333B', dangerSoft:'#FFF1F1', control:'#FFFFFF' },
  dark: { canvas:'#111111', panel:'#191919', group:'#222222', line:'#3A3A3A', border:'#606060', ink:'#EEEEEE', secondary:'#B4B4B4', muted:'#9B9B9B', outside:'#9B9B9B', weekend:'#222222', accent:'#53B9AB', soft:'#3A3A3A', selectedBorder:'#7B7B7B', coral:'#E5484D', success:'#8AD6A6', successSoft:'#1D3025', danger:'#FF9DA1', dangerSoft:'#392327', control:'#2A2A2A' },
  async page(name) {
    let page = penpotUtils.getPageByName(name)||penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id)).find(p=>p.getPluginData('lightcal-page-key')===name);
    if (!page) { page=penpot.createPage(); page.name=name; page.setPluginData('lightcal-refinement',this.version); }
    if(page.getPluginData('lightcal-refinement')!==this.version) throw new Error('Page is not owned by this revision: '+name);
    if(!page.getPluginData('lightcal-page-key'))page.setPluginData('lightcal-page-key',name);
    await penpot.openPage(page);
    this.currentPage=page;
    return page;
  },
  board(name,x,y,w,h,theme='light',radius=12) {
    const t=this[theme];
    let b=this.currentPage.root.children.find(s=>s.type==='board'&&(s.name===name||s.getPluginData('lightcal-board-key')===name));
    if(b) {
      if(!b.getPluginData('lightcal-theme'))throw new Error('Board is not owned by this revision: '+name);
      for(const child of [...b.children])child.remove();
      this.boards=this.boards.filter(item=>item.id!==b.id);
    } else b=penpot.createBoard();
    b.name=name; b.resize(w,h); b.x=x; b.y=y; b.borderRadius=radius;
    b.fills=[{fillColor:t.canvas,fillOpacity:1}];
    b.strokes=[{strokeColor:t.border,strokeWidth:1,strokeAlignment:'inner'}];
    b.setPluginData('lightcal-theme',theme);
    b.setPluginData('lightcal-board-key',name);
    this.boards.push({id:b.id,name,page:this.currentPage.name,pageId:this.currentPage.id,width:w,height:h,theme});
    return b;
  },
  rect(p,name,x,y,w,h,fill,r=0,stroke=null,sw=1) {
    const s=penpot.createRectangle(); s.name=name;s.resize(w,h);s.borderRadius=r;
    s.fills=fill?[{fillColor:fill,fillOpacity:1}]:[];
    s.strokes=stroke?[{strokeColor:stroke,strokeWidth:sw,strokeAlignment:'inner'}]:[];
    p.appendChild(s);penpotUtils.setParentXY(s,x,y);return s;
  },
  fontFamily(parent,value,size) {
    if(/[\u3040-\u30FF\u3400-\u9FFF]/.test(String(value)))return 'Noto Sans JP';
    for(let p=parent;p;p=p.parent)if(p.getPluginData('lightcal-platform')==='macOS'||p.name.startsWith('macOS · '))
      return Number(size)>=20?'SF Pro Display':'SF Pro Text';
    return 'Onest';
  },
  text(p,name,value,x,y,w,h=20,size=13,weight=400,color='#202020',align='left') {
    const s=penpot.createText(String(value));s.name=name;
    const family=this.fontFamily(p,value,size);
    const font=penpot.fonts.all.find(f=>f.name===family);
    if(!font)throw new Error('Exact font family is required: '+family);
    font.applyToText(s,font.variants.find(v=>v.fontWeight===String(weight)&&v.fontStyle==='normal'));
    s.fontSize=String(size);s.fontWeight=String(weight);s.setPluginData('lightcal-font-weight',String(weight));s.setPluginData('lightcal-font-family',family);s.resize(w,h);s.growType='fixed';
    s.fills=[{fillColor:color,fillOpacity:1}];s.align=align;s.verticalAlign='center';
    p.appendChild(s);penpotUtils.setParentXY(s,x,y);return s;
  },
  line(p,name,x,y,w,color) {return this.rect(p,name,x,y,w,1,color);},
  glyph(p,name,x,y,kind,color,size=16) {
    const paths={
      left:'M10 4 6 8l4 4',right:'m6 4 4 4-4 4',down:'m4 6 4 4 4-4',close:'m4 4 8 8M12 4l-8 8',minus:'M4 8h8',
      option:'M3 4h3l4 8h3M10 4h3',shift:'M8 2l6 6h-4v6H6V8H2Z',
      check:'m3 8 3 3 7-7',plus:'M8 3v10M3 8h10',external:'M9 3h4v4M13 3 7 9M6 3H3v10h10v-3',
      general:'M3 4h10M3 8h10M3 12h10M6 2v4M10 6v4M6 10v4',
      accounts:'M10.5 5a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0ZM3 14v-1a5 5 0 0 1 10 0v1',
      appearance:'M8 2a6 6 0 1 0 6 6H8Z',calendars:'M3 4h10v9H3ZM3 6h10M5 2v3M11 2v3',
      about:'M8 7v5M8 4v.2M14 8a6 6 0 1 1-12 0 6 6 0 0 1 12 0',
      settings:'m6 2 4 0 .5 2 2 .8 1.5 1.2-1 2 0 2 1 1.8-2 1.2-2 .5-.5 2H6l-.5-2-2-.5L2 11.8l1-1.8V8L2 6l1.5-1.2 2-.8ZM10 8a2 2 0 1 1-4 0 2 2 0 0 1 4 0',
      refresh:'M12.5 5A5 5 0 1 0 13 10M13 2v4H9',lock:'M4 7h8v7H4ZM5 7V5a3 3 0 0 1 6 0v2',
    };
    const s=penpot.createShapeFromSvg(`<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 16 16"><path d="${paths[kind]||paths.calendars}" fill="none" stroke="${color}" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>`);
    s.name=name;p.appendChild(s);penpotUtils.setParentXY(s,x,y);return s;
  },
  button(p,name,label,x,y,w,t,primary=false,h=28) {
    const s=this.rect(p,name,x,y,w,h,primary?'#00796B':t.control,7,primary?null:t.border);
    if(label)this.text(p,name+' label',label,x+8,y,w-16,h,12,500,primary?'#FFFFFF':t.ink,'center');return s;
  },
  select(p,name,label,x,y,w,t) {
    const s=this.button(p,name,'',x,y,w,t);
    this.text(p,name+' value',label,x+12,y,w-40,28,12,500,t.ink);
    this.glyph(p,name+' chevron',x+w-26,y+6,'down',t.secondary);return s;
  },
  shortcutSymbols(p,name,x,y,w,t,label=null) {
    // Onest has no Option/Shift glyphs. Keep native notation as vector symbols.
    this.glyph(p,name+' Option symbol',x+w/2-22,y+7,'option',t.ink,14);
    this.glyph(p,name+' Shift symbol',x+w/2-4,y+7,'shift',t.ink,14);
    const key=label||this.text(p,name+' label','C',x+w/2+14,y,12,28,12,500,t.ink,'center');
    key.characters='C';key.resize(12,28);penpotUtils.setParentXY(key,x+w/2+14,y);
    return key;
  },
  segmentFrame(p,name,x,y,w,count,t,selected=0) {
    const track=this.rect(p,name+' track',x,y,w,28,t===this.light?'#E8E8E8':'#191919',7);
    const slots=[];
    for(let i=0;i<count;i++) {
      const slotW=w/count,sx=x+i*slotW;
      const hit=this.rect(p,name+' option '+i,sx+2,y+2,slotW-4,24,i===selected?t.soft:null,5,i===selected?t.selectedBorder:null);
      hit.setPluginData('lightcal-role','exclusive-option');slots.push({x:sx,width:slotW,hit});
    }
    return {track,slots};
  },
  toggle(p,name,x,y,on,t) {
    const s=this.rect(p,name,x,y,36,22,on?'#00796B':(t===this.dark?'#606060':'#BBBBBB'),11);
    this.rect(p,name+' thumb',x+(on?17:3),y+3,16,16,'#FFFFFF',8);return s;
  },
  badge(p,name,label,x,y,w,t,kind='neutral') {
    const alert=kind==='alert';
    const fill=alert?'#BC333B':(t===this.dark?'#313131':'#E8E8E8');
    const ink=alert?'#FFFFFF':t.ink;
    this.rect(p,name,x,y,w,22,fill,6);
    this.text(p,name+' label',label,x+6,y,w-12,22,10.5,500,ink,'center');
  },
  // Load refinement-tray-calendar-helper.js before building dated specimens.
  dateBadge: prior?.dateBadge,
  brand(p,x,y,size,date=null,mono=false,darkSurface=null) {
    if(date!==null)return this.dateBadge(p,x,y,size,date,mono,darkSurface??(p.getPluginData('lightcal-theme')==='dark'));
    const edge=size<=24?1:Math.max(1,size/80),r=size*.22,cap=Math.round(size*.27);
    const body=this.rect(p,'App icon body',x,y,size,size,'#FFFFFF',r,'#BBBBBB',edge);
    body.setPluginData('lightcal-icon-optics','centered-v1');
    const head=this.rect(p,'Calendar cap',x,y,size,cap,mono?'#262626':'#E5484D',0);
    head.borderRadiusTopLeft=r;head.borderRadiusTopRight=r;
    // Lower corners are square: the cap ends at the seam, inside the body.
    const tile=size*.19,gap=size*.12,left=x+(size-tile*2-gap)/2;
    const top=y+(size>32?cap+(size-cap-(tile*2+gap))/2:size>24?size*.42-1:size*.42);
    for(let row=0;row<2;row++)for(let col=0;col<2;col++)this.rect(p,'Calendar day tile',left+col*(tile+gap),top+row*(tile+gap),tile,tile,row===1&&col===1?'#E5484D':'#313131',size*.035);
  },
  checkbox(p,name,x,y,checked,color,t) {
    const s=this.rect(p,name,x,y,16,16,checked?color:t.control,4,checked?null:t.border);
    if(checked)this.glyph(p,name+' check',x,y,'check','#FFFFFF');return s;
  },
  calendar(theme,x,y,opts={}) {
    const t=this[theme],platform=opts.platform||'Windows',next=opts.next,selected=opts.selected||31,empty=opts.empty;
    const b=this.board(opts.name||`${platform} · Calendar · ${next?'November':'October'} · ${next?12:selected} · ${theme}`,x,y,352,432,theme,12);
    this.text(b,'Displayed month',next?'November 2026':'October 2026',20,16,214,28,18,600,t.ink);
    [['Previous month','left',244],['Next month','right',276],['Open settings','settings',308]].forEach(([name,kind,bx])=>{
      const hit=this.rect(b,name,bx,16,28,28,name==='Open settings'?t.group:null,7);
      this.glyph(b,name+' icon',bx+6,22,kind,t.secondary);
      hit.setPluginData('lightcal-target',name==='Open settings'?platform+'/'+theme+'/General':platform+'/'+theme+'/'+(name==='Next month'?'next':'calendar'));
    });
    this.text(b,'Today date','Saturday, 31 October',20,52,220,22,11.5,400,t.secondary);
    const today=this.button(b,'Go to today','Today',260,50,76,t,false,26);
    today.setPluginData('lightcal-target',platform+'/'+theme+'/calendar');
    const gx=40,cw=42,rh=36,gy=112;
    for(let c=5;c<7;c++)this.rect(b,'Tinted weekend column',gx+c*cw+2,84,38,242,t.weekend,8);
    this.text(b,'Week number heading','W',8,85,26,20,9.5,500,t.muted,'center');
    ['Mo','Tu','We','Th','Fr','Sa','Su'].forEach((wd,c)=>this.text(b,'Weekday '+wd,wd,gx+c*cw,85,cw,20,11,500,t.secondary,'center'));
    const dots={1:['#00796B'],6:['#00796B','#BD6A16'],9:['#00796B'],12:['#28774B'],14:['#00796B'],16:['#BD6A16'],20:['#00796B'],22:['#00796B','#BD6A16'],27:['#00796B'],29:['#00796B'],30:['#00796B','#BD6A16','#28774B'],31:['#00796B','#BD6A16']};
    for(let r=0;r<6;r++) {
      this.text(b,'Week '+(next?44+r:40+r),String(next?44+r:40+r),8,gy+r*rh,26,32,9.5,400,t.muted,'center');
      for(let c=0;c<7;c++) {
        const index=r*7+c,raw=index-(next?6:3)+1,days=next?30:31,out=raw<1||raw>days,d=raw<1?(next?31:30)+raw:raw>days?raw-days:raw;
        const isToday=d===31&&(next?out&&raw<1:!out),isSelected=!out&&d===(next?12:selected),cx=gx+c*cw,cy=gy+r*rh;
        const hit=this.rect(b,'Date '+(out?'adjacent month ':next?'November ':'October ')+d,cx+4,cy,34,32,isToday?'#D73840':isSelected?t.soft:null,8,isSelected&&!isToday?t.selectedBorder:null,isSelected?1.5:1);
        if(opts.keyboardFocus&&isSelected)this.rect(b,'Keyboard focus outline',cx+2,cy-2,38,36,null,10,t.accent,2);
        this.text(b,'Day numeral '+d,d,cx,cy+1,cw,25,14,isToday||isSelected?600:500,isToday?'#FFFFFF':out?(t===this.light?'#646464':t.outside):t.ink,'center');
        if(!out&&!empty&&(next?[6,12,20].includes(d):dots[d])) {
          const colors=next?['#00796B']:dots[d],start=cx+(cw-(colors.length*3+(colors.length-1)*3))/2;
          colors.forEach((color,k)=>this.rect(b,'Event indicator',start+k*6,cy+26,3,3,isToday?'#FFFFFF':theme==='dark'?(color==='#00796B'?'#53B9AB':color==='#BD6A16'?'#E9B870':'#8AD6A6'):color,1.5));
        }
        if(!next&&!out&&d===30)hit.setPluginData('lightcal-target',platform+'/'+theme+'/selected');
      }
    }
    this.line(b,'Agenda divider',16,338,320,t.line);
    const date=next?'Thursday, 12 November':selected===30?'Friday, 30 October':'Saturday, 31 October';
    this.text(b,'Selected date',date,20,347,246,22,12,600,t.ink);
    this.text(b,'Event count',empty?'0 events':'2 events',268,349,64,18,10.5,400,t.secondary,'right');
    if(empty) {
      this.text(b,'Empty day','Nothing planned. Enjoy your day.',20,381,312,24,12,400,t.secondary);
    }else {
      const events=selected===30?[['10:00','Architecture sync','#00796B'],['14:30','Dentist appointment','#BD6A16']]:next?[['09:30','Roadmap review','#00796B'],['18:00','Dinner with Alex','#BD6A16']]:[['10:00','Coffee with Maya','#BD6A16'],['16:00','Weekend walk','#28774B']];
      events.forEach(([time,title,color],i)=>{
        const ey=376+i*24;this.rect(b,'Event calendar color',20,ey+4,3,14,color,1.5);
        this.text(b,'Event time',time,32,ey,42,22,11,500,t.secondary);
        this.text(b,'Event title',title,84,ey,248,22,12,400,t.ink);
      });
    }
    if(!opts.name?.startsWith('Language'))this.targets[platform+'/'+theme+'/'+(next?'next':selected===30?'selected':empty?'empty':'calendar')]=b;
    return b;
  },
};
// Archive intact originals once; never remove pages or the owner's original shapes.
if (!penpotUtils.getPages().some(p=>p.name.startsWith('Archive · '))) {
  for(const p of penpotUtils.getPages())if(penpotUtils.getPageById(p.id).getPluginData('lightcal-refinement')!==storage.refinement.version)penpotUtils.getPageById(p.id).name='Archive · '+p.name;
}
return {loaded:storage.refinement.version,archived:penpotUtils.getPages().filter(p=>p.name.startsWith('Archive · ')).length};
