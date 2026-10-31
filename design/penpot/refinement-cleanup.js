// Requires a verified permanent snapshot. Original boards remain recoverable
// through Penpot's normal version history, rather than cluttering active pages.
const label='Before refinement cleanup 2026-10-31';
const versions=await penpot.currentFile.findVersions();
if(!versions.some(v=>v.label===label&&!v.isAutosave))throw new Error('Create the permanent pre-cleanup snapshot first.');
const all=penpotUtils.getPages().map(p=>penpotUtils.getPageById(p.id));
const removed=[];
for(const page of all.filter(p=>p.name.startsWith('Archive · '))){removed.push(page.name);page.remove();}
let proposals=0,strays=0;
for(const page of all.filter(p=>p.getPluginData('lightcal-refinement')==='2026-10-31')) {
  await penpot.openPage(page);
  for(const s of [...page.root.children]) {
    if(s.name.includes('ux-proposal')){s.remove();proposals++;}
    else if(s.type!=='board'){s.remove();strays++;}
    else {
      if(!s.getPluginData('lightcal-board-key'))s.setPluginData('lightcal-board-key',s.name);
      const section=s.getPluginData('lightcal-section');
      if(section==='General')s.name=s.name.replace(' · General · ',' · App · ');
      if(section==='Appearance')s.name=s.name.replace(' · Appearance · ',' · Calendar settings · ');
      if(section==='Calendars')s.name=s.name.replace(' · Calendars · ',' · Accounts · ');
    }
  }
  if(!page.getPluginData('lightcal-page-key'))page.setPluginData('lightcal-page-key',page.name);
  page.name=page.name.replace(/^\d+ · /,'');
}
await storage.refinement.page('00 · Overview');
return {snapshot:label,removedPages:removed.length,removedProposals:proposals,removedStrays:strays,pages:penpotUtils.getPages()};
