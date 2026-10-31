// Authoritative Windows compact calendar and shell specimens.
const r=storage.refinement;
await r.page('01 · Windows calendar');
for(const [j,theme] of ['light','dark'].entries()) {
  if(r.batch&&(j<r.batch.start||j>=r.batch.end))continue;
  const y=j*680,t=r[theme];
  r.calendar(theme,0,y);
  r.calendar(theme,400,y,{selected:30});
  r.calendar(theme,800,y,{next:true});
  r.calendar(theme,1200,y,{empty:true,name:`Windows · Empty day · ${theme}`});
  const shell=r.board(`Windows · Taskbar and tray · ${theme}`,1600,y,940,560,theme,16);
  r.text(shell,'Shell title','Windows, at a glance',28,24,884,36,24,600,t.ink);
  r.text(shell,'Shell description','One compact calendar. One clear date in the tray.',28,70,884,24,13,400,t.secondary);
  // One-pixel-per-logical-pixel samples; the taskbar clock remains a separate item.
  for(const [i,dark] of [false,true].entries()) {
    const yy=132+i*176,bg=dark?'#242427':'#F2F2F4',fg=dark?'#F4F4F6':'#25252A';
    r.rect(shell,'Taskbar '+(dark?'dark':'light'),28,yy,884,64,bg,12);
    r.brand(shell,60,yy+16,32);
    r.rect(shell,'Active taskbar indicator',73,yy+54,6,3,'#777BE2',1.5);
    r.glyph(shell,'Tray overflow',638,yy+24,'down',fg);
    r.brand(shell,680,yy+24,16,'31');
    r.glyph(shell,'System status',726,yy+24,'general',fg);
    r.text(shell,'Native Windows clock','10:42',780,yy+9,108,22,12,500,fg,'right');
    r.text(shell,'Native Windows date','31/10/2026',772,yy+32,116,20,11,400,fg,'right');
    r.text(shell,'App icon caption','App icon',52,yy+76,200,24,12,500,t.ink);
    r.text(shell,'Live date caption','Live date · 16 px',636,yy+76,248,24,12,500,t.ink);
    r.text(shell,'App icon detail','Stable in the taskbar and Start.',52,yy+104,360,20,11,400,t.secondary);
    r.text(shell,'Live date detail','A date badge beside the system clock.',636,yy+104,248,20,11,400,t.secondary);
  }
  r.text(shell,'Windows interaction guidance','Click to open or hide · Right-click for the menu · Esc to close',28,508,884,24,12,400,t.secondary);
}
return r.boards.filter(b=>b.page==='01 · Windows calendar');
