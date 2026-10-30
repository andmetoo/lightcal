// Lightcal Penpot Automated Verification Script
// Verifies all 6 pages, 36 artboards, dimensions, and containment bounds.

const pages = penpotUtils.getPages();
const report = [];

for (const p of pages) {
  const pageObj = penpotUtils.getPageById(p.id);
  const boards = pageObj.root.children.filter(ch => ch.type === 'board');
  const boardEntries = [];

  for (const b of boards) {
    const violations = penpotUtils.analyzeDescendants(b, (root, shape) => {
      return !penpotUtils.isContainedIn(shape, root) ? shape.name : null;
    });
    boardEntries.push({
      id: b.id,
      name: b.name,
      x: b.x,
      y: b.y,
      width: b.width,
      height: b.height,
      childrenCount: b.children.length,
      outsideBoundsCount: violations.length
    });
  }

  report.push({
    pageId: p.id,
    pageName: p.name,
    boardCount: boards.length,
    boards: boardEntries
  });
}

return {
  fileId: penpot.currentFile?.id,
  fileName: penpot.currentFile?.name,
  totalPages: report.length,
  totalBoards: report.reduce((acc, p) => acc + p.boardCount, 0),
  pages: report
};
