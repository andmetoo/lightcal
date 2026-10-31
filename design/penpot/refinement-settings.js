// Generate the final Calendar / Accounts / App settings and separate About panel.
const r=storage.refinement;
if(!r?.settingsContent)throw new Error('Run refinement.js and refinement-dense.js first.');
await r.page('02 · Windows settings');
for(const [j,theme]of ['light','dark'].entries())for(const [i,section]of ['Appearance','Calendars','General','About'].entries()) {
  const index=j*4+i;if(r.batch&&(index<r.batch.start||index>=r.batch.end))continue;
  r.settingsContent(section,theme,i*512,j*424);
}
return {page:r.currentPage.name,dimensions:'448 × 360'};
