// Execute in Penpot MCP after opening the page to verify. Read-only audit.
await penpot.waitForLayoutUpdate();
const page=penpot.currentPage,overflows=[],outside=[];
for(const board of page.root.children){
  overflows.push(...penpotUtils.analyzeDescendants(board,(root,shape)=>{
    if(shape.type!=='text')return null;
    const actual=shape.textBounds,box=shape.bounds;
    return actual.width>box.width+1||actual.height>box.height+1
      ?{board:board.name,name:shape.name,text:shape.characters,actual,box}:null;
  }).map(item=>item.result));
  outside.push(...penpotUtils.analyzeDescendants(board,(root,shape)=>
    !penpotUtils.isContainedIn(shape,root)?{board:board.name,name:shape.name}:null
  ).map(item=>item.result));
}
return {page:page.name,boards:page.root.children.length,
  textOverflowCount:overflows.length,textIssues:overflows,
  containmentViolationCount:outside.length,containmentIssues:outside};
