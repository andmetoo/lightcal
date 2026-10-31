// Editable vector specimens, including true 1x tray samples.
const r=storage.refinement;
await r.page('05 · Icons & components');
for(const [j,theme] of ['light','dark'].entries()) {
  const t=r[theme],b=r.board(`Icon system · ${theme}`,0,j*864,1440,800,theme,16);
  r.text(b,'Icon system heading','A small icon with a clear identity',32,24,1376,44,28,600,t.ink);
  r.text(b,'Icon system description','One calendar silhouette. Different drawings for different jobs.',32,82,1376,24,14,400,t.secondary);
  r.brand(b,40,154,128);
  r.text(b,'Application icon heading','Application',204,158,354,28,18,600,t.ink);
  r.text(b,'Application icon description','A stable mark in Start, the taskbar and the Dock.\nFour simple tiles. One intentional accent.',204,200,390,56,13,400,t.secondary);
  r.brand(b,708,154,128,'31');
  r.text(b,'Live icon heading','Today, in the tray',872,158,500,28,18,600,t.ink);
  r.text(b,'Live icon description','A large date, a clean cap, no extra decoration.\nThe numeral changes at local midnight.',872,200,500,56,13,400,t.secondary);
  r.line(b,'Icon comparison divider',32,328,1376,t.line);
  r.text(b,'Small sizes heading','Drawn for the size you actually see',32,354,1376,28,18,600,t.ink);
  for(const [i,size] of [16,20,24,32,48,64].entries()) {
    const xx=32+i*232;
    r.rect(b,'Light surface '+size,xx,408,104,100,'#F2F2F5',12);
    r.rect(b,'Dark surface '+size,xx+112,408,104,100,'#252529',12);
    r.brand(b,xx+(104-size)/2,408+(100-size)/2,size,'31');
    r.brand(b,xx+112+(104-size)/2,408+(100-size)/2,size,'31',true);
    r.text(b,'Size '+size,size+' px',xx,520,216,24,12,500,t.secondary,'center');
  }
  r.text(b,'Variants heading','Same form, even at the extremes',32,578,630,26,18,600,t.ink);
  [1,8,11,28,31].forEach((day,i)=>{
    r.brand(b,44+i*92,636,32,String(day));
    r.text(b,'Date test '+day,String(day),32+i*92,680,56,20,11,400,t.secondary,'center');
  });
  r.text(b,'Menu bar heading','macOS menu bar',744,578,630,26,18,600,t.ink);
  r.rect(b,'Date label sample',744,632,112,42,t.group,8);
  r.text(b,'Date label','31',744,632,112,42,13,600,t.ink,'center');
  r.rect(b,'Weekday date label sample',872,632,136,42,t.group,8);
  r.text(b,'Weekday date label','Sat 31',872,632,136,42,13,600,t.ink,'center');
  r.text(b,'Menu label guidance','System-colored text. No colored tile. No clock duplication.',744,692,630,24,12,400,t.secondary);
  r.text(b,'Icon release note','Design specimens · Native DPI, Explorer and macOS shell checks remain release gates.',32,750,1376,22,11,400,t.secondary);
}
// Standalone export boards have transparent backgrounds and deliberate padding.
let exportY=0;
for(const size of [16,24,32,64,128,256,512,1024]) {
  const b=r.board('App icon export · '+size,1560,exportY,size,size,'light',0);
  b.fills=[];b.strokes=[];
  const pad=size<48?1:Math.round(size/16);
  r.brand(b,pad,pad,size-pad*2);
  exportY+=size+48;
}
return {page:r.currentPage.name,boards:r.boards.filter(b=>b.page===r.currentPage.name).length};
