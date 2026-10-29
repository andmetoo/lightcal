/* Lightcal redesign. Execute through the connected Penpot MCP plugin.
 * Defines helpers only; screens are created explicitly in subsequent calls.
 * No existing shapes are deleted. All event/account data below is synthetic.
 */
storage.lightcal = storage.lightcal || { screens: {}, links: [], pages: {} };
const lc = storage.lightcal;
lc.palette = {
  light: { surface:'#FFFFFF', base:'#F5F7FA', ink:'#172033', muted:'#526077',
    line:'#D8E0EA', weekend:'#EFF1F5', accent:'#1D5FD0', soft:'#EAF2FF',
    onAccent:'#FFFFFF', purple:'#7538C9', amber:'#B07000', danger:'#B42335' },
  dark: { surface:'#182231', base:'#111823', ink:'#F3F6FC', muted:'#B3BED0',
    line:'#344157', weekend:'#263244', accent:'#79B9FF', soft:'#213D5C',
    onAccent:'#10233B', purple:'#C4A3FF', amber:'#FBBF24', danger:'#FFACB6' }
};
lc.font = penpot.fonts.all.find(f=>f.name==='Inter') || penpot.fonts.all.find(f=>f.name==='Noto Sans') || penpot.fonts.findByName('Roboto');
lc.jpFont = penpot.fonts.all.find(f=>f.name==='Noto Sans JP') || lc.font;
lc.board = (parent,name,x,y,w,h,color,r=0) => {
  const b=penpot.createBoard(); b.name=name; b.resize(w,h); b.fills=color?[{fillColor:color,fillOpacity:1}]:[];
  b.borderRadius=r; b.clipContent=true; if(parent)parent.appendChild(b);
  b.x=(parent?.x||0)+x; b.y=(parent?.y||0)+y; return b;
};
lc.rect = (p,name,x,y,w,h,color,r=0,stroke=null) => {
  const s=penpot.createRectangle(); s.name=name; s.resize(w,h); s.fills=color?[{fillColor:color,fillOpacity:1}]:[];
  s.borderRadius=r; if(stroke)s.strokes=[{strokeColor:stroke,strokeWidth:1,strokeAlignment:'inner',strokeStyle:'solid'}];
  p.appendChild(s); s.x=p.x+x; s.y=p.y+y; return s;
};
lc.text = (p,name,value,x,y,w,h,size,color,weight=400,align='left',jp=false) => {
  const s=penpot.createText(value); s.name=name;
  const f=jp?lc.jpFont:lc.font; const variant=f?.variants.find(v=>v.fontWeight===String(weight));
  if(f)f.applyToText(s,variant); s.fontSize=String(size); s.fontWeight=String(weight);
  s.lineHeight='1.35'; s.align=align; s.verticalAlign='center'; s.resize(w,h); s.growType='fixed';
  s.fills=[{fillColor:color,fillOpacity:1}]; p.appendChild(s); s.x=p.x+x; s.y=p.y+y; return s;
};
lc.dot = (p,name,x,y,d,color) => {
  const s=penpot.createEllipse(); s.name=name; s.resize(d,d); s.fills=[{fillColor:color,fillOpacity:1}];
  p.appendChild(s);s.x=p.x+x;s.y=p.y+y;return s;
};
lc.paths = {
  settings:'M4 6H20M4 12H20M4 18H20M9 3V9M15 9V15M9 15V21',
  left:'M14 5L7 12L14 19', right:'M10 5L17 12L10 19', down:'M6 9L12 15L18 9',
  close:'M6 6L18 18M18 6L6 18', minimize:'M5 12H19', check:'M5 12L10 17L19 7',
  external:'M14 4H20V10M20 4L10 14M10 5H5V19H19V14',
  calendar:'M7 3V7M17 3V7M4 10H20M6 5H18Q20 5 20 7V19Q20 21 18 21H6Q4 21 4 19V7Q4 5 6 5Z',
  refresh:'M20 5V11H14M4 19V13H10M5 9A8 8 0 0 1 18 6L20 11M4 13L6 18A8 8 0 0 0 19 15',
  info:'M12 11V17M12 7V7.1M21 12A9 9 0 1 1 3 12A9 9 0 1 1 21 12',
  globe:'M3 12H21M12 3C6 9 6 15 12 21C18 15 18 9 12 3M21 12A9 9 0 1 1 3 12A9 9 0 1 1 21 12',
  sun:'M12 3V1M12 23V21M3 12H1M23 12H21M5 5L3 3M21 21L19 19M5 19L3 21M21 3L19 5M17 12A5 5 0 1 1 7 12A5 5 0 1 1 17 12'
};
lc.icon = (p,key,x,y,size,color) => {
  const s=penpot.createShapeFromSvg(`<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="${lc.paths[key]}" fill="none" stroke="${color}" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>`);
  if(!s)throw new Error('SVG icon creation failed: '+key);s.name='Icon / '+key;s.resize(size,size);p.appendChild(s);s.x=p.x+x;s.y=p.y+y;return s;
};
lc.brand = (p,x,y,size) => {
  const s=lc.rect(p,'Owner-selected Lightcal icon',x,y,size,size,null,0);
  if(lc.image)s.fills=[{fillImage:lc.image,fillOpacity:1}];
  else {s.fills=[{fillColor:'#3B82F6',fillOpacity:1}];lc.icon(p,'calendar',x+size*.2,y+size*.2,size*.6,'#FFFFFF');}
  return s;
};
lc.button = (p,name,label,x,y,w,h,c,kind='secondary',icon=null) => {
  const b=lc.board(p,name,x,y,w,h,kind==='primary'?c.accent:kind==='soft'?c.soft:c.surface,8);
  b.strokes=[{strokeColor:kind==='primary'?c.accent:c.line,strokeWidth:1,strokeAlignment:'inner',strokeStyle:'solid'}];
  if(label)lc.text(b,'Label',label,icon?34:8,0,w-(icon?42:16),h,12,kind==='primary'?c.onAccent:kind==='soft'?c.accent:c.ink,500,icon?'left':'center');
  if(icon)lc.icon(b,icon,label?10:(w-18)/2,(h-18)/2,18,kind==='primary'?c.onAccent:c.muted);
  return b;
};
lc.toggle = (p,name,x,y,on,c) => {
  const b=lc.board(p,name,x,y,42,24,on?c.accent:c.line,12);
  lc.dot(b,'Thumb',on?22:3,3,18,on?c.onAccent:c.surface);return b;
};
lc.line = (p,y,c,x=24,w=p.width-48) => lc.rect(p,'Divider',x,y,w,1,c.line);
lc.link = (shape,to) => { lc.links.push({id:shape.id,to}); return shape; };
lc.page = async name => {
  let p=penpotUtils.getPageByName(name);if(!p){p=penpot.createPage();p.name=name;}
  await penpot.openPage(p);lc.pages[name]=p.id;return p;
};
lc.register=(key,b)=>{lc.screens[key]=b.id;b.setPluginData('lightcal.key',key);return b;};
lc.strings={
  en:{month:'October 2026',next:'November 2026',date:'Thursday, 29 October',short:'Thu, 29 Oct',current:'Current month',today:'Today',week:'Week 44',cal:'2 calendars',selected:'Friday, 30 October',count:'2 events',days:['MO','TU','WE','TH','FR','SA','SU']},
  ru:{month:'Октябрь 2026',next:'Ноябрь 2026',date:'Четверг, 29 октября',short:'Чт, 29 окт.',current:'Текущий месяц',today:'Сегодня',week:'Неделя 44',cal:'2 календаря',selected:'Пятница, 30 октября',count:'2 события',days:['ПН','ВТ','СР','ЧТ','ПТ','СБ','ВС']},
  es:{month:'Octubre de 2026',next:'Noviembre de 2026',date:'Jueves, 29 de octubre',short:'Jue., 29 oct.',current:'Mes actual',today:'Hoy',week:'Semana 44',cal:'2 calendarios',selected:'Viernes, 30 de octubre',count:'2 eventos',days:['LU','MA','MI','JU','VI','SÁ','DO']},
  ja:{month:'2026年10月',next:'2026年11月',date:'10月29日 木曜日',short:'10月29日（木）',current:'今月',today:'今日',week:'第44週',cal:'カレンダー2件',selected:'10月30日 金曜日',count:'予定2件',days:['月','火','水','木','金','土','日']}
};
lc.calendar=(theme,expanded,x,y,options={})=>{
  const c=lc.palette[theme],lang=options.lang||'en',s=lc.strings[lang],jp=lang==='ja';
  const next=!!options.next,mode=expanded?'expanded':'compact',w=expanded?520:352,h=expanded?600:432,pad=expanded?28:20;
  const key=options.key||`${theme}/${mode}/${next?'november':lang}`;
  const b=lc.register(key,lc.board(null,`Calendar / ${expanded?'Taskbar':'Tray'} / ${theme} / ${next?'November':lang.toUpperCase()}`,x,y,w,h,c.surface,expanded?12:16));
  b.strokes=[{strokeColor:c.line,strokeWidth:1,strokeAlignment:'inner',strokeStyle:'solid'}];
  lc.brand(b,pad,16,22);lc.text(b,'App name','Lightcal',pad+30,15,130,24,13,c.ink,600);
  lc.link(lc.button(b,'Open settings','',w-pad-(expanded?102:30),12,30,30,c,'secondary','settings'),`${theme}/settings/General`);
  if(expanded){lc.button(b,'Minimize window','',w-pad-66,12,30,30,c,'secondary','minimize');lc.button(b,'Close window to tray','',w-pad-30,12,30,30,c,'secondary','close');}
  if(expanded){lc.line(b,50,c,0,w);lc.text(b,'Month heading',next?s.next:s.month,pad,68,310,40,27,c.ink,600,'left',jp);}
  else lc.text(b,'Month heading',next?s.next:s.month,pad,62,226,32,20,c.ink,600,'left',jp);
  const navY=expanded?72:63;
  lc.link(lc.button(b,'Previous month','',w-pad-64,navY,30,30,c,'secondary','left'),`${theme}/${mode}/en`);
  lc.link(lc.button(b,'Next month','',w-pad-30,navY,30,30,c,'secondary','right'),`${theme}/${mode}/november`);
  lc.text(b,'Today weekday and date',expanded?s.date:s.short,pad,expanded?111:103,expanded?270:165,22,expanded?13:11,c.muted,400,'left',jp);
  if(!next){
    const bx=expanded?342:184,by=expanded?112:105,bw=expanded?150:148;
    lc.rect(b,'Current month badge',bx,by,bw,20,c.soft,6);
    lc.dot(b,'Current month marker',bx+8,by+7,6,c.accent);
    lc.text(b,'Current month label',s.current,bx+20,by,bw-25,20,10,c.accent,500,'left',jp);
    lc.rect(b,'Current month accent',pad,expanded?104:96,44,2,c.accent,1);
  }
  const headY=expanded?157:145,gridY=expanded?190:174,step=expanded?48:32,cell=expanded?40:30,col=(w-2*pad)/7;
  for(const weekend of [5,6])lc.rect(b,'Gray weekday '+weekend,pad+weekend*col+1,headY-3,col-2,gridY-headY+step*6+3,c.weekend,7);
  s.days.forEach((day,i)=>lc.text(b,'Weekday '+i,day,pad+i*col,headY,col,20,11,c.muted,600,'center',jp));
  for(let i=0;i<42;i++){
    const d=i-(next?5:2),outside=d<1||d>(next?30:31),number=d<1?d+(next?31:30):d>(next?30:31)?d-(next?30:31):d;
    const cx=pad+(i%7)*col,cy=gridY+Math.floor(i/7)*step;
    const today=!next&&d===29,selected=!next&&d===(options.today?29:30);
    const hit=lc.board(b,`Date ${next?'November':'October'} ${number}${outside?' adjacent':''}`,cx,cy,col,cell,null);
    if(today||selected){
      const mark=lc.rect(hit,selected?'Selected date fill':'Today outline',(col-cell)/2,0,cell,cell,selected?c.accent:c.surface,9,selected?null:c.accent);
      if(today&&!selected)mark.strokes=[{strokeColor:c.accent,strokeWidth:1.5,strokeStyle:'solid',strokeAlignment:'inner'}];
    }
    lc.text(hit,'Day number',String(number),0,-1,col,cell,expanded?17:14,selected?c.onAccent:outside?c.muted:c.ink,today||selected?600:400,'center',jp);
    if(!outside&&[2,8,14,20,29,30].includes(d)){
      const count=[14,29,30].includes(d)?2:1;
      for(let j=0;j<count;j++)lc.dot(hit,'Event dot '+j,col/2-1.5+(j-(count-1)/2)*5,cell-5,3,selected?c.onAccent:j?c.purple:c.accent);
    }
    if(!outside)lc.link(hit,`${theme}/${mode}/en`);
  }
  if(expanded){
    lc.line(b,502,c);lc.text(b,'Selection caption','SELECTED DATE',pad,519,250,18,10,c.muted,600);
    lc.text(b,'Selected weekday date',next?(lang==='en'?'Sunday, 1 November':s.next):s.selected,pad,540,300,28,17,c.ink,600,'left',jp);
    lc.text(b,'Selected event count',s.count,pad,572,200,18,11,c.muted,400,'left',jp);
    lc.link(lc.button(b,'Calendar selection',s.cal,338,540,154,34,c,'secondary','calendar'),`${theme}/settings/Calendars`);
    lc.link(lc.button(b,'Return to today',s.today,392,510,100,26,c,'soft'),`${theme}/expanded/en`);
  }else{
    lc.line(b,386,c);lc.link(lc.button(b,'Return to today',s.today,pad,397,80,26,c,'soft'),`${theme}/compact/en`);
    lc.text(b,'ISO week',s.week,116,398,94,24,10,c.muted,400,'center',jp);
    lc.link(lc.button(b,'Calendar selection',s.cal,218,397,114,26,c,'secondary'),`${theme}/settings/Calendars`);
  }
  return b;
};
lc.settings=(theme,tab,x,y,state='normal')=>{
  const c=lc.palette[theme],key=`${theme}/settings/${tab}${state==='normal'?'':'/'+state}`;
  const b=lc.register(key,lc.board(null,`Settings / ${tab} / ${theme}${state==='normal'?'':' / '+state}`,x,y,780,620,c.surface,12));
  b.strokes=[{strokeColor:c.line,strokeWidth:1,strokeStyle:'solid',strokeAlignment:'inner'}];
  lc.brand(b,20,13,22);lc.text(b,'Window title','Lightcal settings',50,12,350,24,13,c.ink,600);
  lc.link(lc.button(b,'Close settings','',730,10,30,30,c,'secondary','close'),`${theme}/expanded/en`);
  lc.line(b,48,c,0,780);lc.rect(b,'Navigation background',0,49,168,571,c.base);
  ['General','Appearance','Calendars','About'].forEach((name,i)=>{
    const active=name===tab;
    const row=lc.board(b,'Navigation / '+name,12,74+i*48,144,38,active?c.soft:null,8);
    lc.icon(row,{General:'settings',Appearance:'sun',Calendars:'calendar',About:'info'}[name],12,10,18,active?c.accent:c.muted);
    lc.text(row,'Navigation label',name,40,0,100,38,12,active?c.accent:c.ink,active?600:400);
    lc.link(row,`${theme}/settings/${name}`);
  });
  lc.text(b,'Settings footer','Lightcal',24,568,120,20,11,c.muted,500);
  const panel=lc.board(b,'Content / '+tab,196,76,556,516,null);
  lc.text(panel,'Section title',tab,0,0,540,40,26,c.ink,600);
  const desc={General:'A few preferences. Your calendar stays close.',Appearance:'Make the calendar comfortable to read.',Calendars:'Choose which events appear in your calendar.',About:'A little calendar. Right where you need it.'};
  lc.text(panel,'Section introduction',desc[tab],0,45,540,24,13,c.muted);
  const field=(name,label,value,yy)=>{lc.text(panel,name+' label',label,0,yy,270,34,13,c.ink,500);const v=lc.button(panel,name,value,300,yy,256,34,c);lc.icon(v,'down',228,9,16,c.muted);return v;};
  if(tab==='General'){
    const lang=field('Language selector','Language','English',100);lc.link(lang,`${theme}/settings/General/languages`);
    lc.text(panel,'Available languages','English · Русский · Español · 日本語',0,146,556,24,12,c.muted,400,'left',true);
    lc.line(panel,190,c,0,556);
    lc.text(panel,'Startup label','Start with Windows',0,209,460,28,14,c.ink,500);
    lc.text(panel,'Startup description','Open quietly in the tray after sign-in.',0,243,460,24,12,c.muted);
    lc.toggle(panel,'Startup off by default',514,214,false,c);lc.line(panel,289,c,0,556);
    lc.text(panel,'Shortcut label','Global shortcut',0,311,500,28,14,c.ink,500);
    lc.button(panel,'Shortcut field',state==='conflict'?'Ctrl + Alt + C':'Not set',0,351,416,38,c);
    lc.link(lc.button(panel,'Record shortcut','Record',432,351,124,38,c),`${theme}/settings/General/conflict`);
    lc.text(panel,'Shortcut explanation',state==='conflict'?'This shortcut is already in use. Choose another.':'Open or hide the tray calendar from anywhere.',0,401,556,34,12,state==='conflict'?c.danger:c.muted);
    lc.text(panel,'Preferences persistence','Changes are saved automatically.',0,480,556,24,11,c.muted);
    if(state==='languages'){
      const menu=lc.board(panel,'Language choices',300,139,256,184,c.surface,9);menu.strokes=[{strokeColor:c.line,strokeWidth:1,strokeStyle:'solid',strokeAlignment:'inner'}];
      ['English','Русский','Español','日本語'].forEach((label,i)=>{
        const row=lc.board(menu,'Language / '+label,4,4+i*44,248,42,i===0?c.soft:null,6);
        lc.text(row,'Language name',label,14,0,194,42,13,i===0?c.accent:c.ink,i===0?600:400,'left',i===3);
        if(i===0)lc.icon(row,'check',218,13,16,c.accent);
        lc.link(row,`${theme}/settings/General`);
      });
    }
  }
  if(tab==='Appearance'){
    lc.text(panel,'Theme label','Theme',0,100,556,26,14,c.ink,500);
    const segment=lc.board(panel,'Theme choices',0,140,556,40,c.base,8);
    const flex=segment.addFlexLayout();flex.dir='row';flex.columnGap=4;flex.horizontalPadding=4;flex.verticalPadding=4;flex.alignItems='center';
    ['Light','System','Dark'].forEach(label=>{const q=lc.button(segment,'Theme '+label,label,0,0,180,32,c,label==='System'?'soft':'secondary');if(label!=='System')lc.link(q,`${label.toLowerCase()}/settings/Appearance`);});
    field('First weekday','First day of week','Monday',215);
    lc.line(panel,273,c,0,556);lc.text(panel,'Gray weekdays label','Gray weekdays',0,293,556,28,14,c.ink,500);
    lc.text(panel,'Gray weekdays helper','Shade any days you choose. Dates remain selectable.',0,326,556,24,12,c.muted);
    const row=lc.board(panel,'Weekday shading choices',0,366,556,38,null);const fx=row.addFlexLayout();fx.dir='row';fx.columnGap=8;fx.alignItems='center';
    ['Mon','Tue','Wed','Thu','Fri','Sat','Sun'].forEach((label,i)=>lc.button(row,'Gray weekday '+label,label,0,0,72,36,c,i>=5?'soft':'secondary'));
    lc.text(panel,'Show weekday label','Show weekday in date',0,433,460,28,13,c.ink,500);lc.toggle(panel,'Show weekday on',514,435,true,c);
    lc.text(panel,'Week numbers label','Show week numbers',0,478,460,28,13,c.ink,500);lc.toggle(panel,'Week numbers on in this example',514,480,true,c);
  }
  if(tab==='Calendars'){
    lc.rect(panel,'Provider summary',0,96,556,78,c.base,10);lc.icon(panel,'calendar',16,120,24,c.accent);
    lc.text(panel,'Provider name','Windows calendars',54,106,340,26,14,c.ink,600);
    lc.text(panel,'Provider access','Read-only access',54,136,340,20,11,c.muted);
    lc.link(lc.button(panel,'Refresh calendars','',506,119,32,32,c,'secondary','refresh'),`${theme}/settings/Calendars`);
    if(state==='denied'||state==='empty'){
      lc.icon(panel,'info',250,226,32,c.muted);
      lc.text(panel,'Unavailable heading',state==='denied'?'Calendar access is unavailable':'No calendars found',24,282,508,34,20,c.ink,600,'center');
      lc.text(panel,'Unavailable explanation','You can still use Lightcal. No events are shown.',24,327,508,48,13,c.muted,400,'center');
      lc.link(lc.button(panel,'Retry provider','Try again',192,399,172,36,c,'soft'),`${theme}/settings/Calendars`);
    }else{
      [['Work','Windows · Work',c.accent,true],['Personal','Windows · Personal',c.purple,true],['Holidays','Windows · Subscribed calendar',c.muted,false]].forEach(([name,source,color,on],i)=>{
        const yy=205+i*82;lc.rect(panel,'Select '+name,0,yy+10,22,22,on?c.accent:c.surface,6,on?null:c.line);
        if(on)lc.icon(panel,'check',3,yy+13,16,c.onAccent);lc.dot(panel,'Calendar color '+name,40,yy+17,8,color);
        lc.text(panel,'Calendar name '+name,name,62,yy,460,28,14,c.ink,500);
        lc.text(panel,'Calendar source '+name,source,62,yy+31,460,20,11,c.muted);lc.line(panel,yy+66,c,0,556);
      });
      lc.text(panel,'Selection explanation','Only checked calendars contribute event dots.',0,480,556,24,12,c.muted);
    }
  }
  if(tab==='About'){
    lc.brand(panel,0,105,72);lc.text(panel,'Product name','Lightcal',94,105,440,38,30,c.ink,600);
    lc.text(panel,'Version label','Development build',94,147,440,24,12,c.muted);
    const github=lc.button(panel,'GitHub repository','GitHub',0,218,268,44,c,'secondary','external');github.addInteraction('click',{type:'open-url',url:'https://github.com/andmetoo/lightcal'});
    const site=lc.button(panel,'Author website','koona.app',284,218,272,44,c,'secondary','external');site.addInteraction('click',{type:'open-url',url:'https://koona.app'});
    lc.text(panel,'License','GPL-3.0-only · Open source',0,284,556,24,12,c.muted);lc.line(panel,332,c,0,556);
    lc.text(panel,'Updates title','Updates',0,354,556,28,16,c.ink,600);
    lc.text(panel,'Updates helper',state==='checking'?'Checking GitHub for a stable release…':state==='available'?'A new version is available.':'Checks only when you ask.',0,393,556,26,13,c.muted);
    const update=lc.button(panel,'Check updates',state==='checking'?'Checking…':state==='available'?'View release on GitHub':'Check for updates',0,443,556,40,c,'soft',state==='available'?'external':'refresh');
    if(state==='available')update.addInteraction('click',{type:'open-url',url:'https://github.com/andmetoo/lightcal/releases'});
    else lc.link(update,`${theme}/settings/About/checking`);
    lc.text(panel,'Manual install note','No automatic downloads, installation, or restart.',0,493,556,20,11,c.muted);
  }
  return b;
};
lc.trayMenu=(theme,x,y)=>{
  const c=lc.palette[theme],b=lc.register(`${theme}/tray-menu`,lc.board(null,`Tray menu / ${theme}`,x,y,256,236,c.surface,10));
  lc.brand(b,16,14,24);lc.text(b,'App name','Lightcal',48,13,180,26,14,c.ink,600);lc.line(b,53,c,12,232);
  [['Open calendar','expanded/en'],['Settings','settings/General'],['About Lightcal','settings/About'],['Quit',null]].forEach(([label,to],i)=>{
    const row=lc.board(b,'Tray menu / '+label,8,64+i*40,240,36,null,6);lc.text(row,'Menu label',label,12,0,212,36,13,c.ink);
    if(to)lc.link(row,`${theme}/${to}`);
  });return b;
};
lc.wire=(start=0,limit=12)=>{
  let linked=0,noOp=0;const missing=[];
  for(const link of lc.links.slice(start,start+limit)){
    const s=penpotUtils.findShapeById(link.id),dest=penpotUtils.findShapeById(lc.screens[link.to]);
    if(!s||!dest){missing.push(link.to);continue;}
    const visited=new Set();let ancestor=s,self=false;
    while(ancestor&&!visited.has(ancestor.id)){
      visited.add(ancestor.id);if(ancestor.id===dest.id){self=true;break;}
      if(ancestor.id==='00000000-0000-0000-0000-000000000000')break;
      ancestor=ancestor.parent;
    }
    if(self){noOp++;continue;}
    s.interactions.filter(i=>i.trigger==='click').forEach(i=>s.removeInteraction(i));
    s.addInteraction('click',{type:'navigate-to',destination:dest});linked++;
  }
  return {linked,noOp,missing:[...new Set(missing)],next:start+limit,total:lc.links.length};
};
lc.wireCurrentPage=()=>{
  const page=penpot.currentPage,rootId=page.root.id,roots=new Set(page.root.children.map(s=>s.id));
  let linked=0;const seen=new Set();
  for(const link of lc.links){
    if(seen.has(link.id))continue;seen.add(link.id);
    const s=page.getShapeById(link.id),dest=page.getShapeById(lc.screens[link.to]);
    if(!s||!dest||!roots.has(dest.id)||s.name.startsWith('Date '))continue;
    let top=s;
    for(let depth=0;depth<12;depth++){
      const parent=top.parent;if(!parent||parent.id===rootId||parent.id===top.id)break;top=parent;
    }
    if(top.id===dest.id)continue;
    s.interactions.filter(i=>i.trigger==='click').forEach(i=>s.removeInteraction(i));
    s.addInteraction('click',{type:'navigate-to',destination:dest});linked++;
  }
  return {page:page.name,linked,crossPage:'Not bound; navigate pages in the editor'};
};
lc.overview=()=>{
  const c=lc.palette.light,b=lc.board(null,'Lightcal / Calendar overview',0,0,2048,840,'#EDF1F6',0);
  lc.brand(b,40,28,40);lc.text(b,'Brand heading','Lightcal',96,24,700,44,32,c.ink,600);
  lc.text(b,'Design proposition','Your month, a click away.',96,73,800,26,15,c.muted);
  lc.rect(b,'Dark theme backdrop',1024,126,984,674,'#0D1420',24);
  lc.text(b,'Light appearance caption','LIGHT',40,112,400,22,10,c.muted,600);
  lc.text(b,'Dark appearance caption','DARK',1064,138,400,22,10,'#B3BED0',600);
  const screens=[lc.calendar('light',false,40,178),lc.calendar('light',true,432,178),
                 lc.calendar('dark',false,1064,178),lc.calendar('dark',true,1456,178)];
  // Prototype destinations must be top-level boards. Keep separate presentation
  // copies inside the overview and the canonical windows to its right.
  screens.forEach(s=>{const preview=s.clone();preview.name=s.name+' / presentation';b.appendChild(preview);preview.showInViewMode=false;s.x+=3520;s.y-=178;});
  lc.text(b,'Review note','Illustrative events · Windows calendar access is subject to the platform probe',40,802,1960,24,12,c.muted);
  lc.register('overview',b);return b;
};
lc.guide=(x,y)=>{
  const c=lc.palette.light,b=lc.register('design-guide',lc.board(null,'Design system / Controls and states',x,y,1640,650,'#F5F7FA',0));
  lc.text(b,'Guide title','Small details. Consistent behavior.',32,24,1550,46,30,c.ink,600);
  lc.text(b,'Guide introduction','8 px rhythm · 12–16 px body text · 32–40 px controls · 2 px visible keyboard focus',32,83,1550,28,14,c.muted);
  for(const [i,theme] of ['light','dark'].entries()){
    const p=lc.palette[theme],panel=lc.board(b,theme+' controls',32+i*804,144,772,450,p.surface,14);
    lc.text(panel,'Palette caption',theme==='light'?'Light surfaces':'Dark surfaces',24,18,720,32,20,p.ink,600);
    lc.button(panel,'Primary button','Continue',24,79,150,38,p,'primary');lc.button(panel,'Secondary button','Cancel',190,79,150,38,p);
    const focus=lc.button(panel,'Keyboard focused button','Today',356,79,150,38,p,'soft');
    const ring=lc.rect(panel,'Visible keyboard focus',353,76,156,44,null,11,p.accent);ring.strokes=[{strokeColor:p.accent,strokeWidth:2,strokeAlignment:'inner',strokeStyle:'solid'}];
    lc.text(panel,'Toggle labels','On',24,148,60,24,12,p.muted);lc.toggle(panel,'Toggle on',88,147,true,p);
    lc.text(panel,'Toggle labels off','Off',190,148,60,24,12,p.muted);lc.toggle(panel,'Toggle off',254,147,false,p);
    const colors=[['Surface',p.surface],['Text',p.ink],['Muted',p.muted],['Accent',p.accent],['Weekdays',p.weekend],['Events',p.purple]];
    colors.forEach(([label,color],j)=>{lc.rect(panel,'Swatch '+label,24+j*120,218,96,56,color,8,p.line);lc.text(panel,'Swatch label '+label,label,24+j*120,285,104,24,11,p.muted);});
    lc.rect(panel,'Error notice',24,347,724,70,p.base,10,p.line);
    lc.text(panel,'Error title','This shortcut is already in use.',40,359,690,25,13,p.danger,600);
    lc.text(panel,'Error helper','Choose another combination. Your previous shortcut stays active.',40,386,690,20,11,p.muted);
  }
  return b;
};
return {ready:true,font:lc.font?.name,japaneseFont:lc.jpFont?.name,screenCount:Object.keys(lc.screens).length};
