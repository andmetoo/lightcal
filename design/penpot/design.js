// Lightcal Penpot Core Library (storage.lc)
// Inspired by Linear Desktop, Raycast, and Notion Calendar (Cron)

storage.lc = {
  THEMES: {
    light: {
      mode: 'light',
      label: 'Light',
      stage: '#EAEAEF',
      canvas: '#FFFFFF',
      panel: '#F7F7F8',
      weekendBand: '#F8F8FA',
      control: '#FFFFFF',
      controlSubtle: '#F2F2F5',
      group: '#F4F4F6',
      controlHover: '#EFEFF2',
      borderWindow: '#DFDFE4',
      hairline: '#ECECEF',
      borderControl: '#E2E2E6',
      ink: '#141416',
      secondary: '#64646C',
      muted: '#A0A0AB',
      outside: '#C8C8D0',
      todayAccent: '#E5484D',
      primaryAccent: '#5E6AD2',
      primarySoft: '#EEF0FF',
      dotWork: '#5E6AD2',
      dotPersonal: '#F59E0B',
      dotHoliday: '#10B981',
      danger: '#E5484D',
      dangerSoft: '#FFF0F0',
      warning: '#D97706',
      warningSoft: '#FEF3C7'
    },
    dark: {
      mode: 'dark',
      label: 'Dark',
      stage: '#08080A',
      canvas: '#101012',
      panel: '#141417',
      weekendBand: '#151519',
      control: '#1E1E22',
      controlSubtle: '#18181C',
      group: '#18181C',
      controlHover: '#26262B',
      borderWindow: '#27272A',
      hairline: '#1F1F24',
      borderControl: '#2C2C32',
      ink: '#F4F4F6',
      secondary: '#94949E',
      muted: '#52525B',
      outside: '#3A3A42',
      todayAccent: '#E5484D',
      primaryAccent: '#5E6AD2',
      primarySoft: '#1D203A',
      dotWork: '#6E7AE2',
      dotPersonal: '#FBBF24',
      dotHoliday: '#34D399',
      danger: '#F87171',
      dangerSoft: '#2A1618',
      warning: '#FBBF24',
      warningSoft: '#272012'
    }
  },

  openPage(name) {
    const p = penpotUtils.getPageByName(name);
    if (!p) throw new Error('Page not found: ' + name);
    penpot.openPage(p);
    return p;
  },

  clearCurrentPage() {
    const children = [...penpot.root.children];
    for (const ch of children) ch.remove();
  },

  createBoard(name, x, y, w, h, fill, stroke, rx = 12) {
    const b = penpot.createBoard();
    b.name = name;
    b.x = x;
    b.y = y;
    b.resize(w, h);
    b.borderRadius = rx;
    b.fills = fill ? [{ fillColor: fill, fillOpacity: 1 }] : [];
    if (stroke) {
      b.strokes = [{ strokeColor: stroke, strokeWidth: 1, strokeAlignment: 'inner' }];
    }
    return b;
  },

  rect(parent, name, rxRel, ryRel, w, h, fill, stroke = null, radius = 0, strokeWidth = 1) {
    const r = penpot.createRectangle();
    r.name = name;
    r.resize(w, h);
    if (radius) r.borderRadius = radius;
    r.fills = fill ? [{ fillColor: fill, fillOpacity: 1 }] : [];
    if (stroke) {
      r.strokes = [{ strokeColor: stroke, strokeWidth, strokeAlignment: 'inner' }];
    }
    parent.appendChild(r);
    penpotUtils.setParentXY(r, rxRel, ryRel);
    return r;
  },

  text(parent, name, str, rxRel, ryRel, size = 12, weight = '500', color = '#141416', fontFamily = 'Inter') {
    const s = String(str);
    const t = penpot.createText(s);
    t.name = name;
    t.fontFamily = fontFamily;
    t.fontSize = String(size);
    t.fontWeight = String(weight);
    t.fills = [{ fillColor: color, fillOpacity: 1 }];
    parent.appendChild(t);
    const estW = Math.max(24, Math.ceil(s.length * size * 0.62) + 8);
    const estH = Math.max(14, Math.ceil(size * 1.35));
    t.resize(estW, estH);
    t.growType = 'fixed';
    t.align = 'left';
    t.verticalAlign = 'center';
    penpotUtils.setParentXY(t, rxRel, ryRel);
    return t;
  },

  centeredText(parent, name, str, boxX, boxY, boxW, boxH, size = 12, weight = '600', color = '#141416', fontFamily = 'Inter') {
    const t = penpot.createText(String(str));
    t.name = name;
    t.fontFamily = fontFamily;
    t.fontSize = String(size);
    t.fontWeight = String(weight);
    t.fills = [{ fillColor: color, fillOpacity: 1 }];
    parent.appendChild(t);
    t.resize(boxW, boxH);
    t.growType = 'fixed';
    t.align = 'center';
    t.verticalAlign = 'center';
    penpotUtils.setParentXY(t, boxX, boxY);
    return t;
  }
};

const lc = storage.lc;

lc.OCTOBER_2026 = {
  title: 'October 2026',
  isCurrentMonth: true,
  currentLabel: 'Current month',
  todayLabel: 'Today',
  weekdays: ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'],
  weeks: [
    { w: 'W40', days: [
      { d: 28, out: true }, { d: 29, out: true }, { d: 30, out: true },
      { d: 1, dots: ['work'] }, { d: 2 }, { d: 3 }, { d: 4 }
    ]},
    { w: 'W41', days: [
      { d: 5 }, { d: 6, dots: ['work', 'personal'] }, { d: 7 },
      { d: 8 }, { d: 9, dots: ['work'] }, { d: 10 }, { d: 11 }
    ]},
    { w: 'W42', days: [
      { d: 12, dots: ['holiday'] }, { d: 13 }, { d: 14, dots: ['work'] },
      { d: 15 }, { d: 16, dots: ['personal'] }, { d: 17 }, { d: 18 }
    ]},
    { w: 'W43', days: [
      { d: 19 }, { d: 20, dots: ['work'] }, { d: 21 },
      { d: 22, dots: ['work', 'personal'] }, { d: 23 }, { d: 24 }, { d: 25 }
    ]},
    { w: 'W44', days: [
      { d: 26 }, { d: 27, dots: ['work'] }, { d: 28 },
      { d: 29, today: true, dots: ['work', 'personal'] },
      { d: 30, selected: true, dots: ['work', 'personal', 'holiday'] },
      { d: 31 }, { d: 1, out: true }
    ]},
    { w: 'W45', days: [
      { d: 2, out: true }, { d: 3, out: true }, { d: 4, out: true },
      { d: 5, out: true }, { d: 6, out: true }, { d: 7, out: true }, { d: 8, out: true }
    ]}
  ],
  compactSummaryHeader: 'FRI, OCT 30 · 2 EVENTS',
  expandedSummaryHeader: 'Friday, October 30',
  expandedCountBadge: '3 events',
  events: [
    { time: '10:00', end: '11:00', title: 'Architecture sync', cal: 'Work', kind: 'work', loc: 'Linear Meet' },
    { time: '14:30', end: '15:15', title: 'Dentist checkup', cal: 'Personal', kind: 'personal', loc: 'Downtown Clinic' },
    { time: 'All day', end: '', title: 'Founder summit prep', cal: 'Holidays', kind: 'holiday', loc: 'Note' }
  ]
};

lc.NOVEMBER_2026 = {
  title: 'November 2026',
  isCurrentMonth: false,
  currentLabel: '',
  todayLabel: 'Today',
  weekdays: ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'],
  weeks: [
    { w: 'W44', days: [
      { d: 26, out: true }, { d: 27, out: true }, { d: 28, out: true },
      { d: 29, out: true }, { d: 30, out: true }, { d: 31, out: true }, { d: 1 }
    ]},
    { w: 'W45', days: [
      { d: 2, dots: ['work'] }, { d: 3 }, { d: 4, dots: ['work'] },
      { d: 5 }, { d: 6, dots: ['personal'] }, { d: 7 }, { d: 8 }
    ]},
    { w: 'W46', days: [
      { d: 9 }, { d: 10, dots: ['work'] }, { d: 11, dots: ['holiday'] },
      { d: 12, selected: true, dots: ['work', 'personal'] }, { d: 13 }, { d: 14 }, { d: 15 }
    ]},
    { w: 'W47', days: [
      { d: 16 }, { d: 17 }, { d: 18, dots: ['work'] },
      { d: 19 }, { d: 20, dots: ['work'] }, { d: 21 }, { d: 22 }
    ]},
    { w: 'W48', days: [
      { d: 23 }, { d: 24, dots: ['personal'] }, { d: 25 },
      { d: 26, dots: ['holiday'] }, { d: 27 }, { d: 28 }, { d: 29 }
    ]},
    { w: 'W49', days: [
      { d: 30, dots: ['work'] }, { d: 1, out: true }, { d: 2, out: true },
      { d: 3, out: true }, { d: 4, out: true }, { d: 5, out: true }, { d: 6, out: true }
    ]}
  ],
  compactSummaryHeader: 'THU, NOV 12 · 2 EVENTS',
  expandedSummaryHeader: 'Thursday, November 12',
  expandedCountBadge: '2 events',
  events: [
    { time: '09:30', end: '10:30', title: 'Q4 roadmap review', cal: 'Work', kind: 'work', loc: 'Room 4B' },
    { time: '18:00', end: '19:30', title: 'Dinner with Alex', cal: 'Personal', kind: 'personal', loc: 'Soho Bistro' }
  ]
};

lc.drawCalendar = function(container, ox, oy, opts) {
  const t = lc.THEMES[opts.theme || 'light'];
  const expanded = !!opts.expanded;
  const data = opts.data || lc.OCTOBER_2026;
  const font = opts.fontFamily || 'Inter';
  const W = expanded ? 520 : 352;
  const H = expanded ? 600 : 432;

  if (opts.drawSurface) {
    lc.rect(container, 'Window Surface', ox, oy, W, H, t.canvas, t.borderWindow, 12, 1);
  }

  const titleY = expanded ? 16 : (data.isCurrentMonth ? 11 : 17);
  const titleX = expanded ? 20 : 16;
  lc.text(container, 'Month Title', data.title, ox + titleX, oy + titleY, expanded ? 16 : 15, '700', t.ink, font);

  if (data.isCurrentMonth) {
    if (expanded) {
      lc.rect(container, 'Current Month Underline', ox + 20, oy + 40, 44, 2, t.primaryAccent, null, 1);
      lc.rect(container, 'Current Month Pill', ox + 138, oy + 16, 98, 22, t.controlSubtle, t.hairline, 11, 1);
      lc.rect(container, 'Current Month Dot', ox + 146, oy + 24, 6, 6, t.primaryAccent, null, 3);
      lc.text(container, 'Current Month Label', data.currentLabel || 'Current month', ox + 156, oy + 20, 10, '500', t.secondary, font);
    } else {
      lc.rect(container, 'Current Month Underline', ox + 16, oy + 34, 28, 2, t.primaryAccent, null, 1);
      lc.rect(container, 'Current Month Dot', ox + 50, oy + 32, 5, 5, t.primaryAccent, null, 2.5);
      lc.text(container, 'Current Month Label', data.currentLabel || 'Current month', ox + 59, oy + 27, 10, '500', t.secondary, font);
    }
  }

  const todayW = opts.todayWidth || 62;
  let gearX = expanded ? 422 : (W - 14 - 26);
  if (expanded) {
    lc.rect(container, 'Win Divider', ox + 458, oy + 20, 1, 14, t.hairline);
    lc.text(container, 'Win Minimize', '—', ox + 470, oy + 18, 11, '500', t.muted, 'Inter');
    lc.text(container, 'Win Close', '×', ox + 494, oy + 16, 13, '500', t.muted, 'Inter');
  }
  const nextX = gearX - 30;
  const todayX = nextX - 4 - todayW;
  const prevX = todayX - 30;

  lc.rect(container, 'Prev Button', ox + prevX, oy + 14, 26, 26, t.control, t.borderControl, 6, 1);
  lc.text(container, 'Prev Icon', '‹', ox + prevX + 10, oy + 18, 13, '600', t.secondary, 'Inter');

  lc.rect(container, 'Today Button', ox + todayX, oy + 14, todayW, 26, t.control, t.borderControl, 6, 1);
  lc.rect(container, 'Today Button Dot', ox + todayX + 8, oy + 24, 6, 6, t.todayAccent, null, 3);
  lc.text(container, 'Today Button Label', data.todayLabel || 'Today', ox + todayX + 18, oy + 20, 11, '600', t.ink, font);

  lc.rect(container, 'Next Button', ox + nextX, oy + 14, 26, 26, t.control, t.borderControl, 6, 1);
  lc.text(container, 'Next Icon', '›', ox + nextX + 10, oy + 18, 13, '600', t.secondary, 'Inter');

  lc.rect(container, 'Settings Button', ox + gearX, oy + 14, 26, 26, t.controlSubtle, t.hairline, 6, 1);
  lc.text(container, 'Settings Icon', '⚙', ox + gearX + 7, oy + 19, 12, '500', t.secondary, 'Inter');

  lc.rect(container, 'Header Hairline', ox, oy + 52, W, 1, t.hairline);

  const gridX = expanded ? 64 : 44;
  const colW = expanded ? 62 : 42;
  const rowStartY = expanded ? 92 : 90;
  const rowH = 44;

  lc.rect(container, 'Weekend Band Sa', ox + gridX + 5 * colW + 2, oy + 60, colW - 4, 292, t.weekendBand, null, 8);
  lc.rect(container, 'Weekend Band Su', ox + gridX + 6 * colW + 2, oy + 60, colW - 4, 292, t.weekendBand, null, 8);

  lc.text(container, 'Week Col Header', 'W', ox + (expanded ? 28 : 18), oy + 66, 10, '600', t.muted, 'Inter');
  for (let c = 0; c < 7; c++) {
    const wd = data.weekdays[c];
    const wx = ox + gridX + c * colW + Math.round(colW / 2) - 8;
    lc.text(container, 'Weekday ' + wd, wd, wx, oy + 65, 11, '500', c >= 5 ? t.muted : t.secondary, font);
  }

  const dotColor = (kind, onToday) => {
    if (onToday) return '#FFFFFF';
    if (kind === 'work') return t.dotWork;
    if (kind === 'personal') return t.dotPersonal;
    return t.dotHoliday;
  };

  for (let r = 0; r < 6; r++) {
    const wk = data.weeks[r];
    const ry = oy + rowStartY + r * rowH;
    lc.text(container, 'Week ' + wk.w, wk.w, ox + (expanded ? 22 : 14), ry + 11, 10, '500', t.muted, 'Inter');

    for (let c = 0; c < 7; c++) {
      const cell = wk.days[c];
      const cx = ox + gridX + c * colW;
      const tileW = expanded ? 42 : 34;
      const tileH = expanded ? 36 : 34;
      const tileX = cx + Math.round((colW - tileW) / 2);
      const tileY = ry + 1;

      if (cell.today) {
        lc.rect(container, 'Today Tile ' + cell.d, tileX, tileY, tileW, tileH, t.todayAccent, null, 8);
      } else if (cell.selected) {
        lc.rect(container, 'Selected Tile ' + cell.d, tileX, tileY, tileW, tileH, t.primarySoft, t.primaryAccent, 8, 1.5);
      }

      const numStr = String(cell.d);
      const numX = cx + Math.round(colW / 2) - (numStr.length > 1 ? 7 : 4);
      const numY = tileY + 7;
      let numColor = t.ink;
      let numWeight = '500';
      if (cell.today) {
        numColor = '#FFFFFF';
        numWeight = '700';
      } else if (cell.selected) {
        numColor = t.ink;
        numWeight = '700';
      } else if (cell.out) {
        numColor = t.outside;
        numWeight = '400';
      } else if (c >= 5) {
        numColor = t.secondary;
      }

      lc.text(container, 'Day ' + numStr, numStr, numX, numY, 12, numWeight, numColor, 'Inter');

      if (cell.dots && cell.dots.length) {
        const n = cell.dots.length;
        const totalDotsW = n * 4 + (n - 1) * 3;
        let dx = cx + Math.round((colW - totalDotsW) / 2);
        const dy = tileY + tileH - 7;
        for (let k = 0; k < n; k++) {
          lc.rect(container, 'Dot', dx, dy, 4, 4, dotColor(cell.dots[k], cell.today), null, 2);
          dx += 7;
        }
      }
    }
  }

  if (!expanded) {
    lc.rect(container, 'Footer Hairline', ox, oy + 358, W, 1, t.hairline);
    lc.text(container, 'Summary Header', data.compactSummaryHeader, ox + 16, oy + 368, 10, '600', t.secondary, font);
    lc.text(container, 'Expand Link', 'Expand ↗', ox + 286, oy + 367, 10, '600', t.primaryAccent, 'Inter');

    for (let i = 0; i < Math.min(2, data.events.length); i++) {
      const ev = data.events[i];
      const ey = oy + 388 + i * 20;
      const barCol = ev.kind === 'work' ? t.dotWork : (ev.kind === 'personal' ? t.dotPersonal : t.dotHoliday);
      lc.rect(container, 'Event Bar ' + i, ox + 16, ey + 2, 3, 12, barCol, null, 1.5);
      lc.text(container, 'Event Time ' + i, ev.time, ox + 25, ey + 1, 11, '600', t.secondary, 'Inter');
      lc.text(container, 'Event Title ' + i, ev.title, ox + 66, ey + 1, 11, '500', t.ink, font);
      lc.text(container, 'Event Cal ' + i, ev.cal, ox + 292, ey + 1, 10, '500', t.muted, font);
    }
  } else {
    lc.rect(container, 'Agenda Panel Bg', ox + 1, oy + 364, W - 2, H - 365, t.panel, null, 11);
    lc.rect(container, 'Agenda Top Hairline', ox, oy + 364, W, 1, t.hairline);
    lc.text(container, 'Agenda Date Heading', data.expandedSummaryHeader, ox + 20, oy + 378, 13, '600', t.ink, font);
    lc.rect(container, 'Agenda Count Pill', ox + 158, oy + 376, 58, 20, t.controlSubtle, t.hairline, 10, 1);
    lc.text(container, 'Agenda Count Text', data.expandedCountBadge, ox + 168, oy + 380, 10, '500', t.secondary, font);

    lc.rect(container, 'New Event Button', ox + 404, oy + 373, 96, 26, t.control, t.borderControl, 6, 1);
    lc.text(container, 'New Event Label', '+ New event', ox + 418, oy + 380, 11, '600', t.ink, font);

    for (let i = 0; i < data.events.length; i++) {
      const ev = data.events[i];
      const ry = oy + 408 + i * 52;
      const barCol = ev.kind === 'work' ? t.dotWork : (ev.kind === 'personal' ? t.dotPersonal : t.dotHoliday);
      lc.rect(container, 'Agenda Row ' + i, ox + 20, ry, 480, 44, t.canvas, t.hairline, 8, 1);
      lc.rect(container, 'Agenda Bar ' + i, ox + 28, ry + 8, 3, 28, barCol, null, 1.5);
      lc.text(container, 'Agenda Title ' + i, ev.title, ox + 40, ry + 8, 12, '600', t.ink, font);
      lc.text(container, 'Agenda Time ' + i, ev.time + (ev.end ? ' – ' + ev.end : '') + ' · ' + ev.loc, ox + 40, ry + 25, 11, '400', t.secondary, font);
      lc.rect(container, 'Agenda Tag ' + i, ox + 432, ry + 12, 58, 20, t.controlSubtle, null, 5);
      lc.text(container, 'Agenda Tag Text ' + i, ev.cal, ox + 442, ry + 16, 10, '500', t.secondary, font);
    }

    lc.rect(container, 'Footer Hairline', ox, oy + 572, W, 1, t.hairline);
    lc.rect(container, 'Sync Dot', ox + 20, oy + 584, 6, 6, t.dotHoliday, null, 3);
    lc.text(container, 'Sync Status', 'Synced 2m ago · 3 calendars active', ox + 32, oy + 580, 11, '400', t.muted, font);
    lc.text(container, 'Collapse Action', 'Collapse ↙', ox + 442, oy + 580, 11, '500', t.secondary, font);
  }
};

lc.drawTrayMenu = function(container, ox, oy, themeName) {
  const t = lc.THEMES[themeName];
  const items = [
    { label: 'Open Lightcal', right: 'Alt+C', active: true },
    { label: 'Jump to Today (Oct 29)', right: '⌘T' },
    { divider: true },
    { label: 'Settings…', right: '⌘,' },
    { label: 'Check for updates…', right: '' },
    { divider: true },
    { label: 'Quit Lightcal', right: '⌘Q', danger: true }
  ];
  let cy = oy + 8;
  for (let i = 0; i < items.length; i++) {
    const it = items[i];
    if (it.divider) {
      lc.rect(container, 'Menu Divider ' + i, ox + 10, cy + 4, 220, 1, t.hairline);
      cy += 10;
    } else {
      if (it.active) {
        lc.rect(container, 'Menu Active Row', ox + 6, cy, 228, 30, t.controlSubtle, null, 6);
      }
      const col = it.danger ? t.danger : t.ink;
      lc.text(container, 'Menu Item ' + it.label, it.label, ox + 16, cy + 8, 12, it.active ? '600' : '500', col, 'Inter');
      if (it.right) {
        lc.text(container, 'Menu Shortcut ' + it.label, it.right, ox + 188, cy + 8, 11, '500', t.muted, 'Inter');
      }
      cy += 32;
    }
  }
};

lc.drawDropdown = function(container, x, y, w, valueText, t, active = false) {
  const strokeCol = active ? t.primaryAccent : t.borderControl;
  lc.rect(container, 'Dropdown Box', x, y, w, 30, t.control, strokeCol, 6, active ? 1.5 : 1);
  lc.text(container, 'Dropdown Text', valueText, x + 10, y + 8, 12, '500', t.ink, 'Inter');
  lc.text(container, 'Dropdown Chevron', active ? '▴' : '▾', x + w - 18, y + 8, 11, '600', t.secondary, 'Inter');
};

lc.drawToggle = function(container, x, y, on, t) {
  const trackCol = on ? t.primaryAccent : (t.mode === 'light' ? '#E4E4EA' : '#3A3A42');
  lc.rect(container, 'Toggle Track', x, y, 46, 28, trackCol, null, 14);
  const thumbX = on ? x + 21 : x + 3;
  const thumb = lc.rect(container, 'Toggle Thumb', thumbX, y + 3, 22, 22, '#FFFFFF', null, 11);
  thumb.shadows = [{
    style: 'drop-shadow',
    offsetX: 0,
    offsetY: 1,
    blur: 6,
    spread: 0,
    color: { color: '#141416', opacity: 0.2 }
  }];
};

lc.drawButton = function(container, x, y, w, label, t, kind = 'secondary') {
  let fill = t.control;
  let stroke = t.borderControl;
  let textCol = t.ink;
  if (kind === 'primary') {
    fill = t.primaryAccent;
    stroke = null;
    textCol = '#FFFFFF';
  } else if (kind === 'subtle') {
    fill = t.controlSubtle;
    stroke = t.hairline;
  } else if (kind === 'danger') {
    fill = t.dangerSoft;
    stroke = t.danger;
    textCol = t.danger;
  }
  lc.rect(container, 'Button ' + label, x, y, w, 30, fill, stroke, 6, 1);
  lc.text(container, 'Button Label ' + label, label, x + 12, y + 8, 12, '600', textCol, 'Inter');
};

lc.drawWeekdayPills = function(container, rightX, y, t) {
  const days = [
    { d: 'Mo', on: false },
    { d: 'Tu', on: false },
    { d: 'We', on: false },
    { d: 'Th', on: false },
    { d: 'Fr', on: false },
    { d: 'Sa', on: true },
    { d: 'Su', on: true }
  ];
  const pillW = 28;
  const gap = 4;
  const totalW = days.length * pillW + (days.length - 1) * gap;
  const startX = rightX - totalW;
  for (let i = 0; i < days.length; i++) {
    const item = days[i];
    const px = startX + i * (pillW + gap);
    const fill = item.on ? (t.mode === 'light' ? t.ink : t.primaryAccent) : t.controlSubtle;
    const stroke = item.on ? null : t.hairline;
    const txtCol = item.on ? '#FFFFFF' : t.secondary;
    lc.rect(container, 'Weekday Pill ' + item.d, px, y, pillW, 26, fill, stroke, 6, 1);
    lc.text(container, 'Weekday Pill Text ' + item.d, item.d, px + 6, y + 6, 11, item.on ? '600' : '500', txtCol, 'Inter');
  }
};

lc.drawFlatRow = function(container, y, title, subtitle, t, showDivider = true) {
  lc.text(container, 'Row Title ' + title, title, 240, y + 8, 13, '500', t.ink, 'Inter');
  if (subtitle) {
    lc.text(container, 'Row Sub ' + title, subtitle, 240, y + 27, 11, '400', t.secondary, 'Inter');
  }
  if (showDivider) {
    lc.rect(container, 'Row Hairline ' + title, 240, y + 54, 500, 1, t.hairline);
  }
};

lc.drawSettingsBoard = function(name, x, y, themeName, section, stateVariant = null) {
  const t = lc.THEMES[themeName];
  const b = lc.createBoard(name, x, y, 780, 620, t.canvas, t.borderWindow, 12);

  lc.rect(b, 'Sidebar Surface', 1, 1, 199, 618, t.panel, null, 11);
  lc.rect(b, 'Sidebar Divider', 200, 0, 1, 620, t.hairline);

  lc.rect(b, 'App Icon Badge', 20, 22, 22, 22, t.primaryAccent, null, 6);
  lc.text(b, 'App Icon Numeral', '29', 24, 26, 11, '700', '#FFFFFF', 'Inter');
  lc.text(b, 'App Name', 'Lightcal', 50, 24, 14, '700', t.ink, 'Inter');
  lc.text(b, 'App Subtitle', 'Preferences', 112, 26, 12, '500', t.muted, 'Inter');

  const navItems = ['General', 'Appearance', 'Calendars', 'About'];
  for (let i = 0; i < navItems.length; i++) {
    const item = navItems[i];
    const ny = 68 + i * 38;
    const isActive = item === section;
    if (isActive) {
      lc.rect(b, 'Nav Active ' + item, 12, ny, 176, 36, t.primarySoft, null, 10);
    }
    lc.text(b, 'Nav Label ' + item, item, 28, ny + 9, 13, isActive ? '600' : '500', isActive ? t.ink : t.secondary, 'Inter');
  }

  lc.rect(b, 'Sidebar Footer Hairline', 16, 564, 168, 1, t.hairline);
  lc.text(b, 'Sidebar Back Link', '← Back to Calendar', 20, 580, 12, '500', t.secondary, 'Inter');

  lc.text(b, 'Win Minimize', '—', 728, 18, 12, '500', t.muted, 'Inter');
  lc.text(b, 'Win Close', '×', 752, 16, 14, '500', t.muted, 'Inter');

  const subtitles = {
    General: 'Regional conventions, startup behavior, and global keyboard shortcuts.',
    Appearance: 'Theme mode, density, gray weekday columns, and visual indicators.',
    Calendars: 'Connected local and CalDAV calendar feeds and visibility.',
    About: 'Version status, release channel, and automatic update preferences.'
  };
  lc.text(b, 'Section Heading', section, 240, 26, 18, '700', t.ink, 'Inter');
  lc.text(b, 'Section Subtitle', subtitles[section] || '', 240, 52, 12, '400', t.secondary, 'Inter');
  lc.rect(b, 'Section Header Hairline', 240, 80, 500, 1, t.hairline);

  if (section === 'General') {
    lc.rect(b, 'Region Group', 228, 88, 524, 176, t.group, null, 14);
    lc.rect(b, 'Startup Group', 228, 276, 524, 148, t.group, null, 14);
    lc.drawFlatRow(b, 92, 'Language', 'Interface language for month names, weekdays, and controls', t, true);
    lc.drawDropdown(b, 568, 104, 172, 'English (US)', t, stateVariant === 'lang-open');

    lc.drawFlatRow(b, 152, 'Start of week', 'First column shown in the calendar grid', t, true);
    lc.drawDropdown(b, 568, 164, 172, 'Monday', t, false);

    lc.drawFlatRow(b, 212, 'Show week numbers', 'Display ISO week numbers (W01–W52) along the left margin', t, true);
    lc.drawToggle(b, 704, 229, true, t);

    lc.drawFlatRow(b, 272, 'Launch at login', 'Start Lightcal quietly in the system tray when you sign in', t, true);
    lc.drawToggle(b, 704, 289, true, t);

    lc.drawFlatRow(b, 332, 'Global shortcut', 'Toggle the calendar popover from anywhere on the desktop', t, false);
    if (stateVariant === 'shortcut-conflict') {
      lc.rect(b, 'Shortcut Box Conflict', 568, 344, 172, 30, t.dangerSoft, t.danger, 6, 1.5);
      lc.text(b, 'Shortcut Key Text', 'Alt + Space', 580, 352, 12, '600', t.danger, 'Inter');
      lc.text(b, 'Shortcut Badge', 'Conflict', 688, 353, 10, '600', t.danger, 'Inter');

      lc.rect(b, 'Conflict Banner', 240, 396, 500, 48, t.dangerSoft, t.danger, 8, 1);
      lc.text(b, 'Conflict Title', 'Shortcut unavailable: Alt + Space is already registered by Raycast.', 256, 406, 12, '600', t.danger, 'Inter');
      lc.text(b, 'Conflict Hint', 'Press a new key combination (for example Alt + C) or click Reset.', 256, 424, 11, '400', t.secondary, 'Inter');
      lc.drawButton(b, 652, 405, 76, 'Reset', t, 'secondary');
    } else {
      lc.rect(b, 'Shortcut Recorder', 568, 344, 172, 30, t.controlSubtle, t.borderControl, 6, 1);
      lc.text(b, 'Shortcut Text', 'Alt + C', 580, 352, 12, '600', t.ink, 'Inter');
      lc.text(b, 'Shortcut Hint', 'Record', 690, 353, 10, '500', t.muted, 'Inter');
    }

    if (stateVariant === 'lang-open') {
      lc.rect(b, 'Lang Popover Menu', 568, 138, 172, 136, t.control, t.primaryAccent, 8, 1);
      const langs = [
        { label: 'English (US)', active: true },
        { label: 'Русский', hover: true },
        { label: 'Español' },
        { label: '日本語' }
      ];
      for (let i = 0; i < langs.length; i++) {
        const lg = langs[i];
        const ly = 144 + i * 31;
        if (lg.active || lg.hover) {
          lc.rect(b, 'Lang Option Bg ' + i, 574, ly, 160, 27, lg.active ? t.primarySoft : t.controlSubtle, null, 5);
        }
        lc.text(b, 'Lang Option ' + lg.label, lg.label, 584, ly + 7, 12, lg.active ? '600' : '500', t.ink, lg.label === '日本語' ? 'Noto Sans JP' : 'Inter');
        if (lg.active) {
          lc.text(b, 'Lang Check', '✓', 716, ly + 7, 12, '700', t.primaryAccent, 'Inter');
        }
      }
    }
  } else if (section === 'Appearance') {
    lc.drawFlatRow(b, 92, 'Theme', 'Choose Alabaster Light, Obsidian Dark, or follow system appearance', t, true);
    lc.drawDropdown(b, 568, 104, 172, themeName === 'light' ? 'Light (Alabaster)' : 'Dark (Obsidian)', t, false);

    lc.drawFlatRow(b, 152, 'Default view mode', 'Open tray popover in Compact grid or Expanded agenda view', t, true);
    lc.drawDropdown(b, 568, 164, 172, 'Compact (352 × 432)', t, false);

    lc.drawFlatRow(b, 212, 'Gray days of week', 'Tint non-working weekday columns with a soft vertical band', t, true);
    lc.drawWeekdayPills(b, 740, 226, t);

    lc.drawFlatRow(b, 272, 'Highlight current month', 'Show Indigo underline and status pill when viewing the active month', t, true);
    lc.drawToggle(b, 704, 289, true, t);

    lc.drawFlatRow(b, 332, 'Show event dots on dates', 'Render up to 3 semantic color dots beneath days with scheduled events', t, false);
    lc.drawToggle(b, 704, 349, true, t);
  } else if (section === 'Calendars') {
    if (stateVariant === 'access-denied') {
      lc.rect(b, 'Access Denied Banner', 240, 96, 500, 88, t.dangerSoft, t.danger, 8, 1);
      lc.text(b, 'Denied Title', 'Calendar access is turned off in System Privacy Settings', 258, 110, 13, '600', t.danger, 'Inter');
      lc.text(b, 'Denied Desc', 'Lightcal needs read permission to display your local OS and CalDAV events.', 258, 130, 11, '400', t.secondary, 'Inter');
      lc.drawButton(b, 258, 146, 164, 'Open Privacy Settings', t, 'secondary');
      lc.drawButton(b, 430, 146, 88, 'Retry Sync', t, 'subtle');
    } else if (stateVariant === 'empty') {
      lc.rect(b, 'Empty State Hairline Box', 240, 110, 500, 160, t.panel, t.hairline, 10, 1);
      lc.text(b, 'Empty Title', 'No calendar accounts connected yet', 384, 148, 14, '600', t.ink, 'Inter');
      lc.text(b, 'Empty Sub', 'Connect iCloud, Google, Fastmail CalDAV, or a local .ics feed.', 332, 172, 12, '400', t.secondary, 'Inter');
      lc.drawButton(b, 412, 206, 156, '+ Add Calendar Account', t, 'primary');
    } else {
      lc.text(b, 'Group Subheader', 'ACTIVE CALENDARS (3)', 240, 96, 11, '600', t.secondary, 'Inter');
      lc.drawButton(b, 624, 88, 116, '+ Add Calendar', t, 'secondary');
      lc.rect(b, 'Cal Top Hairline', 240, 126, 500, 1, t.hairline);

      const cals = [
        { name: 'Work · Engineering', sub: 'CalDAV · alex@linear.app · Synced 2m ago', col: t.dotWork, tag: 'CalDAV' },
        { name: 'Personal', sub: 'iCloud · Primary schedule & reminders', col: t.dotPersonal, tag: 'iCloud' },
        { name: 'Public Holidays', sub: 'Subscribed .ics · United States & National', col: t.dotHoliday, tag: 'Read-only' }
      ];
      for (let i = 0; i < cals.length; i++) {
        const c = cals[i];
        const ry = 132 + i * 60;
        lc.rect(b, 'Cal Checkbox ' + c.name, 240, ry + 14, 18, 18, c.col, null, 5);
        lc.text(b, 'Cal Checkmark ' + i, '✓', 244, ry + 16, 11, '700', '#FFFFFF', 'Inter');
        lc.text(b, 'Cal Title ' + c.name, c.name, 268, ry + 10, 13, '600', t.ink, 'Inter');
        lc.text(b, 'Cal Sub ' + c.name, c.sub, 268, ry + 29, 11, '400', t.secondary, 'Inter');
        lc.rect(b, 'Cal Tag ' + c.name, 672, ry + 14, 68, 22, t.controlSubtle, t.hairline, 6, 1);
        lc.text(b, 'Cal Tag Text ' + c.name, c.tag, 682, ry + 19, 10, '500', t.secondary, 'Inter');
        lc.rect(b, 'Cal Row Hairline ' + i, 240, ry + 56, 500, 1, t.hairline);
      }
    }
  } else if (section === 'About') {
    lc.drawFlatRow(b, 92, 'Lightcal Desktop', 'Version 0.4.2 (2026.10) · Native Rust + Slint UI · 64-bit', t, true);
    if (stateVariant === 'checking') {
      lc.rect(b, 'Checking Pill', 576, 104, 164, 30, t.controlSubtle, t.hairline, 6, 1);
      lc.rect(b, 'Checking Dot', 588, 116, 6, 6, t.primaryAccent, null, 3);
      lc.text(b, 'Checking Text', 'Checking updates…', 602, 112, 12, '500', t.secondary, 'Inter');
    } else if (stateVariant === 'update-available') {
      lc.rect(b, 'Update Badge', 612, 106, 128, 26, t.primarySoft, t.primaryAccent, 6, 1);
      lc.text(b, 'Update Badge Text', 'v0.5.0 available', 626, 113, 11, '600', t.primaryAccent, 'Inter');
    } else {
      lc.drawButton(b, 604, 104, 136, 'Check for updates', t, 'secondary');
    }

    if (stateVariant === 'update-available') {
      lc.rect(b, 'Release Card', 240, 158, 500, 86, t.panel, t.primaryAccent, 8, 1);
      lc.text(b, 'Release Title', 'Lightcal v0.5.0 is ready to install', 256, 170, 13, '600', t.ink, 'Inter');
      lc.text(b, 'Release Notes', 'Faster CalDAV background sync, native macOS vibrancy, and localized Japanese grid.', 256, 190, 11, '400', t.secondary, 'Inter');
      lc.drawButton(b, 256, 208, 132, 'Install & Restart', t, 'primary');
      lc.drawButton(b, 396, 208, 104, 'Release notes', t, 'secondary');

      lc.drawFlatRow(b, 256, 'Automatic updates', 'Download and verify signed releases in the background', t, true);
      lc.drawToggle(b, 704, 273, true, t);

      lc.drawFlatRow(b, 316, 'Update channel', 'Stable production builds or early preview releases', t, false);
      lc.drawDropdown(b, 568, 328, 172, 'Stable', t, false);
    } else {
      lc.drawFlatRow(b, 152, 'Automatic updates', 'Download and verify signed releases in the background', t, true);
      lc.drawToggle(b, 704, 169, true, t);

      lc.drawFlatRow(b, 212, 'Update channel', 'Receive verified stable releases or preview builds', t, true);
      lc.drawDropdown(b, 568, 224, 172, 'Stable', t, false);

      lc.drawFlatRow(b, 272, 'License & source', 'MIT License · Built with Rust, Slint, and zero webview bloat', t, false);
      lc.drawButton(b, 628, 284, 112, 'GitHub Repo ↗', t, 'subtle');
    }
  }

  return b;
};


lc.drawAppIcon = function(container, x, y, size, opts = {}) {
  const dark = !!opts.dark;
  const mode = opts.mode || 'date'; // 'date' | 'weekday-date' | 'time-date'
  const headerText = opts.headerText || 'OCT';
  const numText = opts.numText || '29';
  const timeText = opts.timeText || '22:01';
  const rx = Math.max(4, Math.round(size * 0.23));
  const headerH = Math.max(5, Math.round(size * 0.30));

  const bodyFill = dark ? '#18181C' : '#FFFFFF';
  const bodyStroke = dark ? '#2E2E36' : '#DFDFE4';
  const ink = dark ? '#F4F4F6' : '#141416';
  const subInk = dark ? '#94949E' : '#64646C';

  lc.rect(container, 'Icon Body', x, y, size, size, bodyFill, bodyStroke, rx, 1);

  const hRect = lc.rect(container, 'Icon Header', x, y, size, headerH, '#E5484D', null, 0);
  hRect.borderRadiusTopLeft = rx;
  hRect.borderRadiusTopRight = rx;
  hRect.borderRadiusBottomLeft = 0;
  hRect.borderRadiusBottomRight = 0;

  if (size >= 32) {
    lc.rect(container, 'Icon Header Seam', x + 1, y + headerH, size - 2, 1, dark ? '#27272A' : '#ECECEF');
  }

  if (mode === 'date') {
    if (size >= 36) {
      const hFont = Math.max(7, Math.round(size * 0.14));
      lc.text(container, 'Icon Header Text', headerText, x + Math.round(size * 0.33), y + Math.round((headerH - hFont) * 0.45), hFont, '700', '#FFFFFF', 'Inter');
    }
    const nFont = Math.max(9, Math.round(size * 0.44));
    const nx = x + Math.round(size * (numText.length > 1 ? 0.22 : 0.34));
    const ny = y + headerH + Math.round((size - headerH - nFont) * 0.34);
    lc.text(container, 'Icon Date Numeral', numText, nx, ny, nFont, '700', ink, 'Inter');
  } else if (mode === 'weekday-date') {
    const hFont = Math.max(7, Math.round(size * 0.15));
    lc.text(container, 'Icon Weekday Header', headerText, x + Math.round(size * 0.30), y + Math.round((headerH - hFont) * 0.45), hFont, '700', '#FFFFFF', 'Inter');
    const nFont = Math.max(9, Math.round(size * 0.42));
    const nx = x + Math.round(size * 0.22);
    const ny = y + headerH + Math.round((size - headerH - nFont) * 0.26);
    lc.text(container, 'Icon Date Numeral', numText, nx, ny, nFont, '700', ink, 'Inter');
    if (size >= 48) {
      const dotSize = Math.max(4, Math.round(size * 0.06));
      lc.rect(container, 'Icon Event Dot 1', x + Math.round(size * 0.42), y + size - Math.round(size * 0.14), dotSize, dotSize, '#5E6AD2', null, dotSize / 2);
      lc.rect(container, 'Icon Event Dot 2', x + Math.round(size * 0.52), y + size - Math.round(size * 0.14), dotSize, dotSize, '#F59E0B', null, dotSize / 2);
    }
  } else if (mode === 'time-date') {
    const hFont = Math.max(7, Math.round(size * 0.14));
    lc.text(container, 'Icon Day Header', headerText, x + Math.round(size * 0.20), y + Math.round((headerH - hFont) * 0.45), hFont, '700', '#FFFFFF', 'Inter');
    const tFont = Math.max(8, Math.round(size * 0.24));
    lc.text(container, 'Icon Time Text', timeText, x + Math.round(size * 0.16), y + headerH + Math.round(size * 0.12), tFont, '700', ink, 'Inter');
    const sFont = Math.max(7, Math.round(size * 0.13));
    lc.text(container, 'Icon Sub Date', 'OCT ' + numText, x + Math.round(size * 0.26), y + headerH + Math.round(size * 0.44), sFont, '600', subInk, 'Inter');
  }
};
