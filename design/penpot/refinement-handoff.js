// Review entry point, shared primitives and missing shell/compact specimens.
const r=storage.refinement;
if(!r)throw new Error('Run the refinement library first.');
const includes=index=>!r.batch||(index>=r.batch.start&&index<r.batch.end);
if(includes(0)) {
  await r.page('01 · Windows calendar');
  for(const [j,theme]of ['light','dark'].entries()) {
    const t=r[theme],b=r.board(`Windows · Tray menu · ${theme}`,j*304,1360,240,188,theme,10);
    [['Open Lightcal','calendar'],['Today','calendar'],['Settings…','General'],['Quit Lightcal',null]].forEach(([label,target],i)=>{
      const yy=8+i*42;if(i===3)r.line(b,'Quit separator',12,yy-3,216,t.line);
      const hit=r.rect(b,label,8,yy,224,38,null,6);r.text(b,label+' label',label,20,yy,200,38,12,400,t.ink);
      if(target)hit.setPluginData('lightcal-target','Windows/'+theme+'/'+target);
    });
  }
}
if(includes(1)) {
  await r.page('04 · macOS');
  for(const [j,theme]of ['light','dark'].entries()) {
    r.calendar(theme,1800,j*1400,{platform:'macOS',next:true,name:`macOS · Calendar · November · 12 · ${theme}`});
    r.calendar(theme,2200,j*1400,{platform:'macOS',empty:true,name:`macOS · Empty day · ${theme}`});
  }
}
if(includes(2)) {
  await r.page('05 · Icons & components');
  const b=r.board('Shared controls · Light and dark',0,1728,1280,620,'light',16);
  r.rect(b,'Dark component surface',640,0,640,620,r.dark.canvas,16);
  for(const [j,theme]of ['light','dark'].entries()) {
    const t=r[theme],ox=j*640;
    r.text(b,theme+' component title',theme==='light'?'Light · shared controls':'Dark · shared controls',ox+32,24,576,36,24,600,t.ink);
    r.text(b,theme+' component description','The same dimensions, hierarchy and interaction states.',ox+32,72,576,22,12,400,t.secondary);
    r.button(b,theme+' default button','Default action',ox+32,128,156,t);
    r.button(b,theme+' primary button','Primary action',ox+204,128,156,t,true);
    r.select(b,theme+' select','Monday',ox+376,128,228,t);
    r.toggle(b,theme+' switch on',ox+32,198,true,t);r.toggle(b,theme+' switch off',ox+92,198,false,t);
    r.checkbox(b,theme+' checkbox checked',ox+168,201,true,'#5E63D6',t);r.checkbox(b,theme+' checkbox unchecked',ox+208,201,false,'#5E63D6',t);
    r.badge(b,theme+' read-only status','Read-only',ox+280,198,106,t,'success');r.badge(b,theme+' reconnect status','Reconnect',ox+402,198,106,t,'danger');
    ['Default','Hover','Pressed','Focus','Disabled'].forEach((label,i)=>{
      const xx=ox+32+i*116,dark=theme==='dark';
      if(label==='Focus')r.rect(b,theme+' keyboard focus',xx-2,264,108,32,null,9,t.accent,2);
      const fill=label==='Hover'?(dark?'#333339':'#F0F0F5'):label==='Pressed'?(dark?'#3B3B44':'#E5E5ED'):label==='Disabled'?t.group:t.control;
      r.rect(b,theme+' state '+label,xx,266,104,28,fill,7,label==='Disabled'?t.line:t.border);
      r.text(b,theme+' state '+label+' label',label,xx+8,266,88,28,11,500,label==='Disabled'?t.muted:t.ink,'center');
    });
    r.text(b,theme+' focus guidance','28 px controls · 2 px keyboard focus · Hover, pressed and disabled states',ox+32,324,576,24,12,400,t.secondary);
    const colors=[['Canvas',t.canvas],['Group',t.group],['Text',t.ink],['Secondary',t.secondary],['Accent',t.accent],['Today','#D73840']];
    colors.forEach(([label,color],i)=>{const xx=ox+32+i*96;r.rect(b,label+' swatch',xx,390,80,48,color,8,t.border);r.text(b,label+' token',label,xx,448,88,20,10.5,500,t.secondary);r.text(b,label+' value',color,xx,472,88,20,10,400,t.secondary);});
    r.text(b,theme+' typography guidance','Onest · 18/24 titles · 12–14 controls and dates · 11 secondary text',ox+32,548,576,28,12,400,t.secondary);
  }
}
if(includes(3)) {
  await r.page('00 · Overview');
  const t=r.light,b=r.board('Lightcal · Start here',0,0,1152,704,'light',20);
  r.text(b,'Overview heading','Lightcal, refined.',32,24,1088,44,30,600,t.ink);
  r.text(b,'Overview introduction','One compact calendar. Three focused settings tabs. At home on Windows and macOS.',32,78,1088,24,14,400,t.secondary);
  r.text(b,'Calendar specimen label','CALENDAR · 352 × 432',32,130,352,24,11,600,t.secondary);
  r.text(b,'Settings specimen label','SETTINGS · 448 × 360',432,130,448,24,11,600,t.secondary);
  const calendar=r.calendar('light',32,168,{name:'Overview specimen · compact calendar'});
  const appearance=r.settingsContent('Appearance','light',432,168);
  for(const specimen of [calendar,appearance]){b.appendChild(specimen);specimen.name='Overview specimen · '+specimen.name;}
  r.brand(b,960,190,128);
  r.text(b,'Identity heading','A clearer identity',936,354,184,28,17,600,t.ink);
  r.text(b,'Identity explanation','Drawn for 16 px and up.\nA stable app icon.\nA live tray date.',936,396,184,74,12,400,t.secondary);
  r.text(b,'Scope note','Read-only Google OAuth\nFour languages\nLight and dark\nRust + Slint',936,498,184,86,11,400,t.secondary);
  const win=r.button(b,'Review Windows','Review Windows',32,636,168,t,true,36);win.setPluginData('lightcal-target','Windows/light/Appearance');
  const mac=r.button(b,'Review macOS','Review macOS',216,636,168,t,false,36);mac.setPluginData('lightcal-target','macOS/light/Appearance');
  r.text(b,'Review scope','Design prototype · Events and accounts are examples.',432,636,688,36,12,400,t.secondary);
}
return {created:r.currentPage.name,boards:r.boards.length};
