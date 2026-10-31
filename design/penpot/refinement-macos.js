// Run after the Windows pages have been reviewed. Shared Rust/Slint design;
// platform-specific chrome and status-item behavior, not a second visual brand.
const r=storage.refinement;
await r.page('04 · macOS');
const includes=index=>!r.batch||(index>=r.batch.start&&index<r.batch.end);
for(const [j,theme] of ['light','dark'].entries()) {
  const y=j*1400,t=r[theme];
  if(includes(j*7))r.calendar(theme,0,y,{platform:'macOS'});
  if(includes(j*7+1))r.calendar(theme,400,y,{platform:'macOS',selected:30});
  if(includes(j*7+2)) {
  const shell=r.board(`macOS · Menu bar and Dock · ${theme}`,800,y,940,560,theme,16);
  r.text(shell,'macOS presentation heading','At home on macOS',28,24,884,36,24,600,t.ink);
  r.text(shell,'macOS presentation description','The same calendar identity, sized for the native menu bar.',28,70,884,24,13,400,t.secondary);
  for(const [i,dark] of [false,true].entries()) {
    const yy=136+i*170,bg=dark?'#222222':'#F0F0F0',fg=dark?'#F0F0F0':'#262626';
    r.rect(shell,'Native menu bar specimen',28,yy,884,24,bg,4);
    r.text(shell,'Menu bar app name','Lightcal',48,yy,160,24,12,600,fg);
    r.text(shell,'Menu bar menus','File     Edit     View     Window     Help',168,yy,390,24,12,400,fg);
    r.rect(shell,'Selected Lightcal status item',665,yy,24,24,dark?'#606060':'#D9D9D9',4);
    r.dateBadge(shell,668,yy+3,18,'31',false,dark);
    r.glyph(shell,'System status menu',750,yy+4,'general',fg);
    r.text(shell,'Native macOS clock','10:42',810,yy,80,24,12,500,fg,'right');
    r.text(shell,'Menu bar caption','Shared calendar identity',48,yy+58,300,24,13,500,t.ink);
    r.text(shell,'Menu bar detail','18 pt artwork. System-sized button. Color or monochrome.',48,yy+88,590,22,12,400,t.secondary);
  }
  r.rect(shell,'Dock specimen',600,457,312,76,theme==='dark'?'#2A2A2A':'#E8E8E8',20,t.border);
  r.brand(shell,634,471,48);
  r.rect(shell,'Running application indicator',656,523,4,4,t.secondary,2);
  r.text(shell,'Dock identity caption','One recognizable\napplication icon',704,471,184,48,12,500,t.ink);
  }
  for(const [i,section] of ['General','Appearance','Calendars','About'].entries())if(includes(j*7+3+i))r.settingsContent(section,theme,i*512,y+684,'macOS');
}
return {page:r.currentPage.name,boards:r.boards.filter(b=>b.page===r.currentPage.name).length};
