// Independent UX review: a fixed 448 × 360 utility panel with three tabs.
const r=storage.refinement;
r.settingsContent=function(section,theme,x,y,platform='Windows',state='default') {
  const t=this[theme],mac=platform==='macOS',about=section==='About';
  const shortcutConflict=state==='conflict'||state.startsWith('conflict-language-');
  const tab=section==='Appearance'?'Calendar':section==='Calendars'?'Accounts':'App';
  const b=this.board(`${platform} · ${section} · ${state} · ${theme}`,x,y,448,360,theme,mac?12:10);
  b.setPluginData('lightcal-section',section);b.setPluginData('lightcal-state',state);b.setPluginData('lightcal-platform',platform);
  if(mac) {
    ['#FF5F57','#FEBC2E','#28C840'].forEach((c,i)=>this.rect(b,'Window control',14+i*18,14,10,10,c,5));
    this.text(b,'Window title',about?'About Lightcal':'Lightcal settings',88,5,272,28,12,500,t.ink,'center');
  } else {
    this.brand(b,16,8,20);this.text(b,'Brand','Lightcal',44,4,84,28,13,600,t.ink);this.text(b,'Window title',about?'About':'Settings',130,6,210,24,11,400,t.secondary);
    this.glyph(b,'Minimize window',376,10,'minus',t.secondary,14);this.glyph(b,'Close settings',416,10,'close',t.secondary,14);
  }
  if(!about) {
    [['Calendar','Appearance','calendars'],['Accounts','Calendars','accounts'],['App','General','settings']].forEach(([label,target,icon],i)=>{
      const xx=12+i*144,active=tab===label,hit=this.rect(b,'Navigate to '+target,xx,42,136,32,active?t.soft:null,7);
      this.glyph(b,label+' navigation icon',xx+26,51,icon,active?t.ink:t.secondary,14);
      if(active)this.rect(b,'Active tab marker',xx+56,71,24,2,t.accent,1);
      this.text(b,target+' navigation label',label,xx+48,42,82,32,12,active?600:400,active?t.ink:t.secondary);
      hit.setPluginData('lightcal-target',platform+'/'+theme+'/'+target);
    });
    this.line(b,'Navigation divider',16,83,416,t.line);
  }
  const row=(title,index,help)=>{
    const yy=96+index*40;
    const compactError=section==='General'&&shortcutConflict&&index===2;
    const label=this.text(b,title,title,28,yy+(compactError?2:4),184,compactError?20:32,12,500,t.ink);
    if(help)label.setPluginData('lightcal-help',help);
    if(index<4)this.line(b,title+' divider',28,yy+39,392,t.line);
    return yy;
  };
  const setTarget=(s,key)=>s.setPluginData('lightcal-target',platform+'/'+theme+'/'+key);
  const footer=()=>{
    this.line(b,'Footer divider',16,320,416,t.line);
    const link=this.text(b,'About Lightcal link','About Lightcal',16,328,240,24,11,500,t.secondary);setTarget(link,'About');
    this.text(b,'Version','0.2.2',352,328,80,24,10.5,400,t.secondary,'right');
  };
  if(section==='Appearance') {
    this.rect(b,'Calendar preferences',16,96,416,200,t.group,9);
    row('Theme',0);
    this.segmentFrame(b,'Theme selection',216,102,204,3,t);
    ['System','Light','Dark'].forEach((label,i)=>{
      const xx=216+i*68,active=i===0;
      this.text(b,label+' theme label',label,xx,102,68,28,11,active?600:400,t.ink,'center');
    });
    row('Week starts on',1);this.select(b,'Choose first weekday','Monday',284,142,136,t);
    row('Shade days',2);
    ['Mo','Tu','We','Th','Fr','Sa','Su'].forEach((day,i)=>{
      const xx=220+i*29,on=i>=5;this.rect(b,'Shade weekday '+(i+1),xx,182,26,28,on?t.soft:t.control,6,on?t.selectedBorder:t.border);
      this.text(b,'Shade weekday label '+(i+1),day,xx,182,26,28,10.5,on?600:400,on?t.ink:t.secondary,'center');
    });
    row('Week numbers',3);this.toggle(b,'Show week numbers',384,225,true,t);
    row('Event dots',4);this.toggle(b,'Show event dots',384,265,true,t);
    footer();
  } else if(section==='General') {
    this.rect(b,'Application preferences',16,96,416,200,t.group,9);
    row('Language',0);const lang=this.select(b,'Choose language','English',252,102,168,t);setTarget(lang,'language');
    row(mac?'Open at login':'Start with Windows',1);this.toggle(b,'Startup off by default',384,145,false,t);
    row('Keyboard shortcut',2,'Optional. Opens or hides the calendar; conflicts preserve the previous binding.');
    if(shortcutConflict) {
      this.text(b,'Shortcut conflict','Used by another app',28,201,184,14,10.5,400,t.danger);
      if(mac)this.shortcutSymbols(b,'Current shortcut',216,176,112,t);
      else this.text(b,'Shortcut value','Win + Shift + C',216,176,112,40,11,500,t.ink,'right');
      const key=this.button(b,'Record shortcut','Change',336,182,84,t);setTarget(key,'shortcut');
    } else {
      this.text(b,'Shortcut state','Not set',252,176,76,40,11,400,t.secondary);
      const key=this.button(b,'Record shortcut','Record…',336,182,84,t);setTarget(key,'shortcut');
    }
    row(mac?'Menu bar icon':'Tray icon',3);
    {
      this.segmentFrame(b,mac?'Menu label selection':'Tray palette selection',268,222,152,2,t);
      [false,true].forEach((mono,i)=>{
        const xx=268+i*76;this.brand(b,xx+6,228,16,'31',mono);this.text(b,mono?'Monochrome label':'Color label',mono?'Mono':'Color',xx+27,222,42,28,10.5,mono?400:600,t.ink);
      });
    }
    row('Updates',4);const updates=b.children.find(s=>s.name==='Updates');if(updates)setTarget(updates,'About');footer();
    if(state==='language') {
      this.rect(b,'Language menu',252,138,168,154,t.control,8,t.border);
      ['English','Русский','Español','日本語'].forEach((name,i)=>{
        if(i===0)this.rect(b,'Selected language',258,144,156,32,t.soft,5);
        this.text(b,'Language option '+name,name,268,144+i*36,124,32,12,400,t.ink);
      });
    }
  } else if(section==='Calendars') {
    this.text(b,'Accounts heading','Your accounts',16,92,264,28,12,600,t.ink);
    const add=this.button(b,'Add calendar','+ Add account',300,92,132,t);setTarget(add,'add');
    this.rect(b,'Account panel',16,132,416,168,t.group,9);
    const outlook=state.startsWith('outlook'),waiting=state==='signing-in'||state==='outlook-signing';
    const signing=['disconnected','signing-in','cancelled','denied','outlook','outlook-signing'].includes(state);
    const preAccount=state==='disconnected'||state==='outlook';
    if(preAccount) {
      this.text(b,'Account provider','No accounts yet.',28,150,360,24,12.5,600,t.ink);
      this.text(b,'Connection explanation','Add an account to show events.',28,182,392,22,11,400,t.secondary);
    } else {
      this.text(b,'Account provider',outlook?'Microsoft Outlook':'Google Calendar',28,138,240,22,12.5,600,t.ink);
    }
    this.text(b,'Example account',preAccount?'':signing?'Read-only calendar access':'alex@example.com',28,160,280,18,10.5,400,t.secondary);
    if(!preAccount)this.badge(b,'Account state',state==='expired'?'Reconnect':state==='offline'?'Offline':'Read-only',316,141,104,t,state==='expired'?'alert':'neutral');
    if(!preAccount)this.line(b,'Account header divider',28,186,392,t.line);
    if(signing && !preAccount) {
      this.text(b,'Connection explanation',waiting?'Waiting for browser…':state==='cancelled'?'Sign-in cancelled. Your accounts are unchanged.':state==='denied'?'Calendar access was not granted.':'Read your calendars and events in Lightcal.',28,196,392,22,11,400,t.secondary);
      if(!waiting)this.text(b,'Permission limit','No creating, editing, or deleting events.',28,222,392,20,11,400,t.secondary);
      if(!waiting){const connect=this.button(b,'Connect account',state==='cancelled'||state==='denied'?'Try again':outlook?'Continue with Microsoft':'Continue with Google',28,258,190,t,true);setTarget(connect,outlook?'outlook-signing':'signing-in');}
      if(waiting){const cancel=this.button(b,'Cancel account sign-in','Cancel',328,258,92,t);setTarget(cancel,outlook?'Calendars':'cancelled');}
    } else if(state==='no-calendars') {
      this.text(b,'No calendars found','No calendars found for this account.',28,206,392,24,12,500,t.ink);
      this.text(b,'No calendars advice','Try another account or refresh the list.',28,234,392,20,11,400,t.secondary);this.button(b,'Refresh calendars','Refresh',316,260,104,t);
    } else {
      ['Personal','Work','Public holidays'].forEach((name,i)=>{
        const yy=194+i*25;this.checkbox(b,'Show '+name,28,yy+3,state!=='no-selection',['#A45E18','#00796B','#28774B'][i],t);this.text(b,name+' calendar',name,52,yy,348,22,12,400,t.ink);
      });
      const status=state==='expired'?'Events may be out of date.':state==='offline'?'Saved events · Updated 10:42':state==='no-selection'?'Select calendars to show their events.':'Updated just now';
      this.text(b,'Sync status',status,28,274,284,18,10.5,400,state==='expired'?t.danger:t.secondary);
      const manage=this.button(b,'Account action',state==='expired'?'Reconnect':'Manage',328,268,92,t,state==='expired');setTarget(manage,state==='expired'?'signing-in':'manage');
    }
    footer();
    if(state==='manage') {
      this.rect(b,'Account action menu',228,198,192,88,t.control,8,t.border);
      this.text(b,'Refresh calendars action','Refresh calendars',244,204,160,32,12,400,t.ink);
      this.text(b,'Disconnect account action','Disconnect account…',244,244,160,32,12,400,t.danger);
    }
  } else if(about) {
    this.brand(b,24,56,48);this.text(b,'App name','Lightcal',92,55,332,30,22,600,t.ink);this.text(b,'App version','Version 0.2.2',92,90,332,20,11,400,t.secondary);
    this.rect(b,'Update check',16,132,416,64,t.group,9);
    const checking=state==='checking',available=state==='update-available',offline=state==='update-offline';
    this.text(b,'Update status',checking?'Checking for updates…':available?'Version 0.2.3 available':offline?'Couldn’t check for updates':'You’re up to date',28,140,264,24,12,500,t.ink);
    if(!checking)this.text(b,'Update detail',offline?'Check your connection and try again.':available?'See what’s new on GitHub.':'Updates are checked when you ask.',28,166,264,18,10.5,400,t.secondary);
    const check=this.button(b,'Check for updates',checking?'Checking…':available?'View release':'Check now',316,150,104,t,available);if(!checking)setTarget(check,'checking');
    [['GitHub repository','Source code and issues'],['koona.app','Lightcal on the web'],['GPL-3.0-only','License and third-party notices']].forEach(([title,desc],i)=>{
      const yy=208+i*36;this.text(b,title,title,28,yy,192,30,12,500,t.ink);this.text(b,title+' description',desc,220,yy,180,30,10.5,400,t.secondary,'right');this.glyph(b,title+' link',410,yy+8,'external',t.secondary,14);
    });
    this.line(b,'About footer divider',16,320,416,t.line);const back=this.text(b,'Back to settings','‹ Back to settings',16,328,416,24,11,500,t.secondary);setTarget(back,'General');
  }
  const key=platform+'/'+theme+'/'+(state==='default'?section:state==='conflict'?'shortcut':state);this.targets[key]=b;
  return b;
};
return {loaded:true,size:'448 × 360',tabs:['Calendar','Accounts','App']};
