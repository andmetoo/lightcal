/* Load design.js and the owner-selected icon into storage.lightcal first.
 * Execute this file once to define buildNext(), then call it once per batch.
 * Rebuilds the assistant-generated layout on clearly named pages. Existing
 * pages are retained until the replacement is complete and independently checked.
 */
const lc=storage.lightcal;
if(!lc?.font||!lc.image)throw new Error('Load design.js and upload the selected icon first.');
lc.plan=lc.plan||{phase:0,originalPageId:penpot.currentPage.id,
  originalRoots:penpot.root.children.map(s=>({id:s.id,name:s.name,key:s.getPluginData('lightcal.key')}))};
lc.buildNext=async()=>{
  const phase=lc.plan.phase;
  if(phase===0){
    lc.screens={};lc.links=[];await lc.page('00 · Overview');
    lc.overview();
    for(const key of ['light/compact/en','light/expanded/en','dark/compact/en','dark/expanded/en']){
      penpotUtils.findShapeById(lc.screens[key]).remove();delete lc.screens[key];
    }
  }else if(phase===1){
    await lc.page('01 · Calendar');
    for(const [i,theme] of ['light','dark'].entries()){
      lc.calendar(theme,false,i*1040,0);lc.calendar(theme,true,i*1040+400,0);
    }
  }else if(phase===2){
    await lc.page('01 · Calendar');
    for(const [i,theme] of ['light','dark'].entries()){
      lc.calendar(theme,false,i*1040,720,{next:true});lc.calendar(theme,true,i*1040+400,720,{next:true});
      lc.trayMenu(theme,i*400,1450);
    }
  }else if(phase===3||phase===4){
    await lc.page('02 · Settings');
    const theme=phase===3?'light':'dark';
    for(const [i,tab] of ['General','Appearance','Calendars','About'].entries())lc.settings(theme,tab,theme==='light'?0:860,i*700);
  }else if(phase===5||phase===6){
    await lc.page('03 · States');
    const theme=phase===5?'light':'dark',xx=theme==='light'?0:860;
    lc.settings(theme,'General',xx,0,'languages');lc.settings(theme,'General',xx,700,'conflict');
    lc.settings(theme,'Calendars',xx,1400,'denied');lc.settings(theme,'Calendars',xx,2100,'empty');
  }else if(phase===7){
    await lc.page('03 · States');
    for(const [i,theme] of ['light','dark'].entries()){
      lc.settings(theme,'About',i*860,2800,'checking');lc.settings(theme,'About',i*860,3500,'available');
    }
  }else if(phase===8){
    await lc.page('04 · Languages');
    for(const [i,lang] of ['en','ru','es','ja'].entries())lc.calendar('light',false,i*432,0,{lang,key:'locale/'+lang});
  }else if(phase===9){
    await lc.page('05 · Design system');lc.guide(0,0);
    for(const [theme,palette] of Object.entries(lc.palette))for(const [key,value] of Object.entries(palette)){
      const name=`Lightcal/${theme}/${key}`;
      if(!penpot.library.local.colors.some(c=>c.name===name)){const color=penpot.library.local.createColor();color.name=name;color.color=value;}
    }
  }else return {complete:true,pages:penpotUtils.getPages(),screens:lc.screens};
  await penpot.waitForLayoutUpdate();lc.plan.phase++;
  return {completed:phase,next:lc.plan.phase,page:penpot.currentPage.name,screens:Object.keys(lc.screens).length};
};
return {ready:true,next:lc.plan.phase,originalPageId:lc.plan.originalPageId};
