// Platform edge states and the add-calendar flow. All account data is synthetic.
const r=storage.refinement;
const platform=r.statePlatform||'Windows',offset=platform==='macOS'?2240:0;
await r.page('03 · Connections & states');
const start=r.batch?.start??0,end=r.batch?.end??26;
const states=[
  ['Calendars','disconnected','light'],['Calendars','disconnected','dark'],
  ['Calendars','signing-in','light'],['Calendars','signing-in','dark'],
  ['Calendars','cancelled','light'],['Calendars','denied','dark'],
  ['Calendars','expired','light'],['Calendars','expired','dark'],
  ['Calendars','offline','light'],['Calendars','offline','dark'],
  ['Calendars','no-selection','light'],['Calendars','no-selection','dark'],
  ['Calendars','no-calendars','light'],['General','language','light'],
  ['General','conflict','dark'],['About','checking','light'],
  ['About','update-available','dark'],['About','update-offline','light'],
];
for(const [i,[section,state,theme]] of states.entries()) {
  if(i<start||i>=end)continue;
  const b=r.settingsContent(section,theme,offset+(i%4)*512,Math.floor(i/4)*424,platform,state);
  if(state==='conflict')r.targets[platform+'/'+theme+'/shortcut']=b;
}
for(const [i,theme] of ['light','dark'].entries()) {
  if(18+i<start||18+i>=end)continue;
  const t=r[theme],b=r.settingsContent('Calendars',theme,offset+i*512,2120,platform,'add');
  const scrim=r.rect(b,'Modal scrim',0,0,448,360,'#000000',10);scrim.fills=[{fillColor:'#000000',fillOpacity:theme==='light'?.2:.4}];
  r.rect(b,'Add calendar dialog',44,80,360,236,t.canvas,12,t.border);
  r.text(b,'Dialog heading','Add an account',60,94,292,28,18,600,t.ink);
  r.text(b,'Dialog introduction','Choose a read-only source.',60,126,328,20,11,400,t.secondary);
  const close=r.rect(b,'Close dialog',364,92,28,28,null,6);r.glyph(b,'Close dialog icon',370,98,'close',t.secondary);
  close.setPluginData('lightcal-target',platform+'/'+theme+'/Calendars');
  [['Google Calendar','Connect using your Google account.'],['Microsoft Outlook','Work, school, or a personal account.'],['Private ICS link','Subscribe using a calendar address.']].forEach(([title,desc],j)=>{
    const yy=154+j*48;
    const hit=r.rect(b,'Add '+title,60,yy,328,40,t.group,7);
    r.glyph(b,title+' symbol',72,yy+12,'calendars',t.secondary,16);
    r.text(b,title+' name',title,102,yy,254,40,12,500,t.ink);
    r.glyph(b,title+' forward',364,yy+12,'right',t.secondary);
    hit.setPluginData('lightcal-target',platform+'/'+theme+'/'+(j===0?'disconnected':j===2?'ics':'outlook'));
  });
}
for(const [i,theme] of ['light','dark'].entries()) {
  if(20+i<start||20+i>=end)continue;
  const t=r[theme],b=r.settingsContent('Calendars',theme,offset+i*512,2544,platform,'ics');
  const scrim=r.rect(b,'Modal scrim',0,0,448,360,'#000000',10);scrim.fills=[{fillColor:'#000000',fillOpacity:theme==='light'?.2:.4}];
  r.rect(b,'ICS dialog',24,92,400,210,t.canvas,12,t.border);
  r.text(b,'ICS heading','Add a calendar link',40,104,320,28,18,600,t.ink);
  const close=r.rect(b,'Close dialog',384,104,28,28,null,6);r.glyph(b,'Close dialog icon',390,110,'close',t.secondary);
  close.setPluginData('lightcal-target',platform+'/'+theme+'/Calendars');
  r.text(b,'ICS privacy explanation','Anyone with the address can read this calendar.',40,140,368,20,11,400,t.secondary);
  r.text(b,'ICS address label','Private ICS address',40,168,368,18,11,500,t.secondary);
  r.rect(b,'ICS URL input',40,192,368,32,t.control,7,t.border);
  r.text(b,'ICS URL placeholder','https://example.com/calendar.ics',50,192,348,32,11,400,t.secondary);
  r.text(b,'ICS storage explanation','Stays on this computer.',40,234,368,18,10.5,400,t.secondary);
  const cancel=r.button(b,'Cancel ICS','Cancel',208,258,88,t);cancel.setPluginData('lightcal-target',platform+'/'+theme+'/Calendars');
  r.button(b,'Subscribe to ICS','Subscribe',308,258,100,t,true);
}
for(const [j,[state,theme]]of [['outlook','light'],['outlook','dark'],['outlook-signing','light'],['outlook-signing','dark']].entries()) {
  if(22+j<start||22+j>=end)continue;
  r.settingsContent('Calendars',theme,offset+j*512,2968,platform,state);
}
return {page:r.currentPage.name,boards:r.currentPage.root.children.filter(s=>s.type==='board').length};
