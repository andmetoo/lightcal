// Lightcal Penpot Page Builders (00 · Overview through 05 · Design system)
// Execute after loading design.js into storage.lc

const lc = storage.lc;

// Ensure all 6 pages exist
const pageNames = [
  '00 · Overview',
  '01 · Calendar',
  '02 · Settings',
  '03 · States',
  '04 · Languages',
  '05 · Design system'
];
const existing = penpotUtils.getPages();
if (existing.length > 0 && existing[0].name === 'Page 1') {
  penpotUtils.getPageById(existing[0].id).name = pageNames[0];
}
for (const name of pageNames) {
  if (!penpotUtils.getPageByName(name)) {
    const p = penpot.createPage();
    p.name = name;
  }
}

// 00 · Overview
lc.openPage('00 · Overview');
lc.clearCurrentPage();
{
  const b = lc.createBoard('Overview · Light & Dark Showcase', 0, 0, 2060, 860, '#111114', '#27272A', 16);
  lc.text(b, 'Overview Title', 'Lightcal — Native Desktop Calendar (2026 Visual System)', 32, 26, 20, '700', '#F4F4F6', 'Inter');
  lc.text(b, 'Overview Subtitle', 'Alabaster & Ink (Light) · Obsidian Graphite (Dark) · Coral Today (#E5484D) · Linear Indigo (#5E6AD2) · Borderless Grid & Flat Hairline Settings', 32, 54, 12, '400', '#94949E', 'Inter');

  lc.rect(b, 'Light Stage Mat', 24, 92, 992, 740, lc.THEMES.light.stage, '#D4D4DA', 14, 1);
  lc.text(b, 'Light Stage Label', 'LIGHT THEME · ALABASTER & INK (#FFFFFF / #F7F7F8)', 52, 118, 11, '700', '#64646C', 'Inter');
  lc.text(b, 'Light Compact Caption', 'Compact Tray Popover · 352 × 432', 52, 162, 11, '600', '#64646C', 'Inter');
  lc.drawCalendar(b, 52, 186, { theme: 'light', expanded: false, drawSurface: true });
  lc.text(b, 'Light Expanded Caption', 'Expanded Agenda Window · 520 × 600', 444, 162, 11, '600', '#64646C', 'Inter');
  lc.drawCalendar(b, 444, 186, { theme: 'light', expanded: true, drawSurface: true });

  lc.rect(b, 'Dark Stage Mat', 1044, 92, 992, 740, lc.THEMES.dark.stage, '#222226', 14, 1);
  lc.text(b, 'Dark Stage Label', 'DARK THEME · OBSIDIAN GRAPHITE (#101012 / #141417)', 1072, 118, 11, '700', '#94949E', 'Inter');
  lc.text(b, 'Dark Compact Caption', 'Compact Tray Popover · 352 × 432', 1072, 162, 11, '600', '#94949E', 'Inter');
  lc.drawCalendar(b, 1072, 186, { theme: 'dark', expanded: false, drawSurface: true });
  lc.text(b, 'Dark Expanded Caption', 'Expanded Agenda Window · 520 × 600', 1464, 162, 11, '600', '#94949E', 'Inter');
  lc.drawCalendar(b, 1464, 186, { theme: 'dark', expanded: true, drawSurface: true });
}

// 01 · Calendar (10 boards)
lc.openPage('01 · Calendar');
lc.clearCurrentPage();
{
  const b1 = lc.createBoard('Calendar · Compact · Light · Current Month', 0, 0, 352, 432, lc.THEMES.light.canvas, lc.THEMES.light.borderWindow, 12);
  lc.drawCalendar(b1, 0, 0, { theme: 'light', expanded: false, data: lc.OCTOBER_2026 });

  const b2 = lc.createBoard('Calendar · Expanded · Light · Current Month', 400, 0, 520, 600, lc.THEMES.light.canvas, lc.THEMES.light.borderWindow, 12);
  lc.drawCalendar(b2, 0, 0, { theme: 'light', expanded: true, data: lc.OCTOBER_2026 });

  const b3 = lc.createBoard('Calendar · Compact · Light · Next Month', 968, 0, 352, 432, lc.THEMES.light.canvas, lc.THEMES.light.borderWindow, 12);
  lc.drawCalendar(b3, 0, 0, { theme: 'light', expanded: false, data: lc.NOVEMBER_2026 });

  const b4 = lc.createBoard('Calendar · Expanded · Light · Next Month', 1368, 0, 520, 600, lc.THEMES.light.canvas, lc.THEMES.light.borderWindow, 12);
  lc.drawCalendar(b4, 0, 0, { theme: 'light', expanded: true, data: lc.NOVEMBER_2026 });

  const b5 = lc.createBoard('Tray Menu · Light', 1936, 0, 240, 216, lc.THEMES.light.canvas, lc.THEMES.light.borderWindow, 10);
  lc.drawTrayMenu(b5, 0, 0, 'light');

  const b6 = lc.createBoard('Calendar · Compact · Dark · Current Month', 0, 660, 352, 432, lc.THEMES.dark.canvas, lc.THEMES.dark.borderWindow, 12);
  lc.drawCalendar(b6, 0, 0, { theme: 'dark', expanded: false, data: lc.OCTOBER_2026 });

  const b7 = lc.createBoard('Calendar · Expanded · Dark · Current Month', 400, 660, 520, 600, lc.THEMES.dark.canvas, lc.THEMES.dark.borderWindow, 12);
  lc.drawCalendar(b7, 0, 0, { theme: 'dark', expanded: true, data: lc.OCTOBER_2026 });

  const b8 = lc.createBoard('Calendar · Compact · Dark · Next Month', 968, 660, 352, 432, lc.THEMES.dark.canvas, lc.THEMES.dark.borderWindow, 12);
  lc.drawCalendar(b8, 0, 0, { theme: 'dark', expanded: false, data: lc.NOVEMBER_2026 });

  const b9 = lc.createBoard('Calendar · Expanded · Dark · Next Month', 1368, 660, 520, 600, lc.THEMES.dark.canvas, lc.THEMES.dark.borderWindow, 12);
  lc.drawCalendar(b9, 0, 0, { theme: 'dark', expanded: true, data: lc.NOVEMBER_2026 });

  const b10 = lc.createBoard('Tray Menu · Dark', 1936, 660, 240, 216, lc.THEMES.dark.canvas, lc.THEMES.dark.borderWindow, 10);
  lc.drawTrayMenu(b10, 0, 0, 'dark');
}

// 02 · Settings (8 boards)
lc.openPage('02 · Settings');
lc.clearCurrentPage();
{
  lc.drawSettingsBoard('Settings · General · Light', 0, 0, 'light', 'General');
  lc.drawSettingsBoard('Settings · Appearance · Light', 828, 0, 'light', 'Appearance');
  lc.drawSettingsBoard('Settings · Calendars · Light', 1656, 0, 'light', 'Calendars');
  lc.drawSettingsBoard('Settings · About · Light', 2484, 0, 'light', 'About');

  lc.drawSettingsBoard('Settings · General · Dark', 0, 668, 'dark', 'General');
  lc.drawSettingsBoard('Settings · Appearance · Dark', 828, 668, 'dark', 'Appearance');
  lc.drawSettingsBoard('Settings · Calendars · Dark', 1656, 668, 'dark', 'Calendars');
  lc.drawSettingsBoard('Settings · About · Dark', 2484, 668, 'dark', 'About');
}

// 03 · States (12 boards)
lc.openPage('03 · States');
lc.clearCurrentPage();
{
  lc.drawSettingsBoard('State · General · Languages Dropdown Open · Light', 0, 0, 'light', 'General', 'lang-open');
  lc.drawSettingsBoard('State · General · Shortcut Conflict · Light', 828, 0, 'light', 'General', 'shortcut-conflict');
  lc.drawSettingsBoard('State · Calendars · Access Denied · Light', 1656, 0, 'light', 'Calendars', 'access-denied');
  lc.drawSettingsBoard('State · Calendars · Empty · Light', 2484, 0, 'light', 'Calendars', 'empty');
  lc.drawSettingsBoard('State · About · Checking Updates · Light', 3312, 0, 'light', 'About', 'checking');
  lc.drawSettingsBoard('State · About · Update Available · Light', 4140, 0, 'light', 'About', 'update-available');

  lc.drawSettingsBoard('State · General · Languages Dropdown Open · Dark', 0, 668, 'dark', 'General', 'lang-open');
  lc.drawSettingsBoard('State · General · Shortcut Conflict · Dark', 828, 668, 'dark', 'General', 'shortcut-conflict');
  lc.drawSettingsBoard('State · Calendars · Access Denied · Dark', 1656, 668, 'dark', 'Calendars', 'access-denied');
  lc.drawSettingsBoard('State · Calendars · Empty · Dark', 2484, 668, 'dark', 'Calendars', 'empty');
  lc.drawSettingsBoard('State · About · Checking Updates · Dark', 3312, 668, 'dark', 'About', 'checking');
  lc.drawSettingsBoard('State · About · Update Available · Dark', 4140, 668, 'dark', 'About', 'update-available');
}

// 04 · Languages (4 boards)
lc.openPage('04 · Languages');
lc.clearCurrentPage();
{
  const baseWeeks = lc.OCTOBER_2026.weeks;
  const enData = lc.OCTOBER_2026;
  const ruData = {
    ...lc.OCTOBER_2026,
    title: 'Октябрь 2026',
    currentLabel: 'Текущий месяц',
    todayLabel: 'Сегодня',
    weekdays: ['Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб', 'Вс'],
    weeks: baseWeeks,
    compactSummaryHeader: 'ПТ, 30 ОКТ · 2 СОБЫТИЯ',
    events: [
      { time: '10:00', title: 'Архитектурный синк', cal: 'Работа', kind: 'work' },
      { time: '14:30', title: 'Визит к стоматологу', cal: 'Личное', kind: 'personal' }
    ]
  };
  const esData = {
    ...lc.OCTOBER_2026,
    title: 'Octubre 2026',
    currentLabel: 'Mes actual',
    todayLabel: 'Hoy',
    weekdays: ['Lu', 'Ma', 'Mi', 'Ju', 'Vi', 'Sá', 'Do'],
    weeks: baseWeeks,
    compactSummaryHeader: 'VIE, 30 OCT · 2 EVENTOS',
    events: [
      { time: '10:00', title: 'Revisión de arquitectura', cal: 'Trabajo', kind: 'work' },
      { time: '14:30', title: 'Consulta dental', cal: 'Personal', kind: 'personal' }
    ]
  };
  const jaData = {
    ...lc.OCTOBER_2026,
    title: '2026年 10月',
    currentLabel: '今月',
    todayLabel: '今日',
    weekdays: ['月', '火', '水', '木', '金', '土', '日'],
    weeks: baseWeeks,
    compactSummaryHeader: '10月30日 (金) · 2件の予定',
    events: [
      { time: '10:00', title: 'アーキテクチャ定例', cal: '仕事', kind: 'work' },
      { time: '14:30', title: '歯科検診', cal: '個人', kind: 'personal' }
    ]
  };

  const l1 = lc.createBoard('Language · English (en-US) · Light', 0, 0, 352, 432, lc.THEMES.light.canvas, lc.THEMES.light.borderWindow, 12);
  lc.drawCalendar(l1, 0, 0, { theme: 'light', expanded: false, data: enData, todayWidth: 62 });

  const l2 = lc.createBoard('Language · Русский (ru-RU) · Dark', 400, 0, 352, 432, lc.THEMES.dark.canvas, lc.THEMES.dark.borderWindow, 12);
  lc.drawCalendar(l2, 0, 0, { theme: 'dark', expanded: false, data: ruData, todayWidth: 74 });

  const l3 = lc.createBoard('Language · Español (es-ES) · Light', 800, 0, 352, 432, lc.THEMES.light.canvas, lc.THEMES.light.borderWindow, 12);
  lc.drawCalendar(l3, 0, 0, { theme: 'light', expanded: false, data: esData, todayWidth: 52 });

  const l4 = lc.createBoard('Language · 日本語 (ja-JP) · Dark', 1200, 0, 352, 432, lc.THEMES.dark.canvas, lc.THEMES.dark.borderWindow, 12);
  lc.drawCalendar(l4, 0, 0, { theme: 'dark', expanded: false, data: jaData, todayWidth: 56, fontFamily: 'Noto Sans JP' });
}

// 05 · Design system (1 board)
lc.openPage('05 · Design system');
lc.clearCurrentPage();
{
  const b = lc.createBoard('Design System · Tokens, Typography & Native Primitives', 0, 0, 1640, 660, '#101012', '#27272A', 16);
  lc.text(b, 'DS Title', 'Lightcal Design System — Linear / Raycast / Notion Calendar Craft', 32, 24, 18, '700', '#F4F4F6', 'Inter');
  lc.text(b, 'DS Subtitle', '90% quiet monochrome surfaces · 10% intentional Coral (#E5484D) & Indigo (#5E6AD2) accents · Borderless grids · Flat hairline settings', 32, 50, 12, '400', '#94949E', 'Inter');
  lc.rect(b, 'DS Header Hairline', 32, 78, 1576, 1, '#1F1F24');
}


// 06 · Shell, Dock & Icons (3 boards: Windows 11 Taskbar & Tray, macOS Menu Bar & Dock, Icon System & Dynamic Matrix)
lc.openPage('06 · Shell, Dock & Icons');
lc.clearCurrentPage();
{
  const b1 = lc.createBoard('Windows 11 · Taskbar & System Tray Integration', 0, 0, 1440, 620, '#0E0E11', '#27272A', 16);
  const b2 = lc.createBoard('macOS · Menu Bar (Status Bar) & Dock Integration', 0, 668, 1440, 620, '#0E0E11', '#27272A', 16);
  const b3 = lc.createBoard('Lightcal · Icon System & Dynamic Variants', 0, 1336, 1440, 520, '#101012', '#27272A', 16);
}
