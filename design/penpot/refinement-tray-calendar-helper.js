// Load-only helper: defines a function; does not read or mutate shapes/pages.
if(penpot.currentFile?.id!=='c2d27051-3cd1-813e-8008-dd9b45707753'||!storage.refinement)
  throw new Error('Connect Lightcal with the refinement helpers loaded.');
storage.refinement.dateBadge=function(p,x,y,size,date,mono=false,dark=false,existing=null){
  const pad=0,stroke=Math.max(1,size/16),radius=Math.max(2,size*.16);
  const capHeight=Math.max(2,size*.125),inverse=mono&&dark;
  const ink=inverse?'#EEEEEE':'#141414',cap=mono?(dark?'#EEEEEE':'#313131'):'#E5484D';
  // Use nested fills, not a stroke: Penpot PNG exports can center an 'inner'
  // stroke outside the bounds. Both filled contours stay inside the canvas.
  const body=penpot.createShapeFromSvg(`<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}"><rect width="${size}" height="${size}" rx="${radius}" fill="${inverse?'#B4B4B4':'#9B9B9B'}"/><rect x="${stroke}" y="${stroke}" width="${size-2*stroke}" height="${size-2*stroke}" rx="${Math.max(0,radius-stroke)}" fill="${inverse?'#202020':'#FFFFFF'}"/></svg>`);
  body.name='Live date icon body';p.appendChild(body);penpotUtils.setParentXY(body,x+pad,y+pad);
  // The rail follows the outer page arc; there is no decorative binder or seam.
  const offset=radius-Math.sqrt(Math.max(0,radius*radius-(radius-capHeight)**2));
  const d=`M${pad+radius} ${pad}H${size-pad-radius}A${radius} ${radius} 0 0 1 ${size-pad-offset} ${pad+capHeight}H${pad+offset}A${radius} ${radius} 0 0 1 ${pad+radius} ${pad}Z`;
  const head=penpot.createShapeFromSvg(`<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}"><path d="${d}" fill="${cap}"/></svg>`);
  head.name='Live date calendar rail';p.appendChild(head);penpotUtils.setParentXY(head,x,y);
  // Native fontdue fit for all 31 dates (bundled Onest Bold 2.001).
  const fitted={16:11.3327,18:11.4055,20:12.9783,24:16.2490,32:22.6653,36:24.4360,40:27.7067,48:33.9980,64:45.3306};
  const fontSize=fitted[size]??size*.7083;
  let numeral=existing;
  if(!numeral){
    numeral=penpot.createText(String(date));numeral.name='Live date numeral';
    p.appendChild(numeral);
  }
  const font=penpot.fonts.all.find(f=>f.name==='Onest');
  if(!font)throw new Error('Exact Onest is required.');
  font.applyToText(numeral,font.variants.find(v=>v.fontWeight==='700'&&v.fontStyle==='normal'));
  numeral.characters=String(date);numeral.name='Live date numeral';
  numeral.fontWeight='700';numeral.growType='fixed';numeral.align='center';numeral.verticalAlign='center';
  numeral.setPluginData('lightcal-font-family','Onest');numeral.setPluginData('lightcal-font-weight','700');
  if(existing){body.setParentIndex(numeral.parentIndex);head.setParentIndex(numeral.parentIndex);}
  numeral.fontSize=String(fontSize);numeral.resize(size,size-pad*2-capHeight);
  penpotUtils.setParentXY(numeral,x,y+pad+capHeight-size*.02);
  numeral.fills=[{fillColor:ink,fillOpacity:1}];
  numeral.getRange(0,numeral.characters.length).fills=[{fillColor:ink,fillOpacity:1}];
  numeral.setPluginData('lightcal-date-icon','calendar-badge-v3');
  numeral.setPluginData('lightcal-date-icon-origin',JSON.stringify({x,y,size,mono,dark}));
  for(const s of [body,head])s.setPluginData('lightcal-date-icon-part',numeral.id);
  return numeral;
};
return {loaded:'calendar-badge-v3',shapesChanged:0};
