// Layout examples, using synthetic event titles unchanged by UI translation.
const r=storage.refinement;
await r.page('06 · Languages');
const locales=[
  {code:'en',theme:'light',map:{}},
  {code:'ru',theme:'dark',map:{
    'October 2026':'Октябрь 2026','Sat, 31 Oct':'Сб, 31 окт.','Today':'Сегодня','Mo':'Пн','Tu':'Вт','We':'Ср','Th':'Чт','Fr':'Пт','Sa':'Сб','Su':'Вс','Saturday, 31 October':'Суббота, 31 октября','2 events':'2 события',
    'Settings':'Настройки','General':'Основные','Appearance':'Оформление','Calendars':'Календари','About':'О программе','Language & region':'Язык и регион','Language':'Язык','Used for dates and the interface.':'Язык дат и интерфейса.','English':'Русский','First day of the week':'Первый день недели','Weekend shading is set separately.':'Затенение выходных настраивается отдельно.','Monday':'Понедельник','Startup':'Автозапуск','Start with Windows':'Запускать вместе с Windows','Keep Lightcal ready in the system tray.':'Lightcal будет доступен в системном трее.','Keyboard shortcut':'Горячая клавиша','Open or hide Lightcal':'Открыть или скрыть Lightcal','Optional. Choose a shortcut that works for you.':'Необязательно. Выберите удобное сочетание.','Record shortcut':'Задать сочетание','No shortcut set':'Сочетание не задано'}},
  {code:'es',theme:'light',map:{
    'October 2026':'Octubre de 2026','Current month':'Mes actual','Sat, 31 Oct':'Sáb., 31 oct.','Today':'Hoy','Mo':'Lu','Tu':'Ma','We':'Mi','Th':'Ju','Fr':'Vi','Sa':'Sá','Su':'Do','Saturday, 31 October':'Sábado, 31 de octubre','2 events':'2 eventos',
    'Settings':'Ajustes','General':'General','Appearance':'Apariencia','Calendars':'Calendarios','About':'Acerca de','Language & region':'Idioma y región','Language':'Idioma','Used for dates and the interface.':'Para las fechas y la interfaz.','English':'Español','First day of the week':'Primer día de la semana','Weekend shading is set separately.':'El sombreado se configura por separado.','Monday':'Lunes','Startup':'Inicio','Start with Windows':'Iniciar con Windows','Keep Lightcal ready in the system tray.':'Mantener Lightcal en la bandeja del sistema.','Keyboard shortcut':'Atajo de teclado','Open or hide Lightcal':'Mostrar u ocultar Lightcal','Optional. Choose a shortcut that works for you.':'Opcional. Elige una combinación cómoda.','Record shortcut':'Definir atajo','No shortcut set':'Sin atajo asignado'}},
  {code:'ja',theme:'dark',map:{
    'October 2026':'2026年10月','Current month':'今月','Sat, 31 Oct':'10月31日（土）','Today':'今日','Mo':'月','Tu':'火','We':'水','Th':'木','Fr':'金','Sa':'土','Su':'日','Saturday, 31 October':'10月31日 土曜日','2 events':'予定2件',
    'Settings':'設定','General':'一般','Appearance':'外観','Calendars':'カレンダー','About':'このアプリ','Language & region':'言語と地域','Language':'言語','Used for dates and the interface.':'日付と画面表示に使用します。','English':'日本語','First day of the week':'週の開始曜日','Weekend shading is set separately.':'曜日の背景色は別に設定します。','Monday':'月曜日','Startup':'起動','Start with Windows':'Windowsと同時に起動','Keep Lightcal ready in the system tray.':'システムトレイに常駐します。','Keyboard shortcut':'キーボードショートカット','Open or hide Lightcal':'Lightcalを表示・非表示','Optional. Choose a shortcut that works for you.':'必要に応じてキーの組み合わせを設定します。','Record shortcut':'キーを設定','No shortcut set':'未設定'}},
];
for(const [i,locale] of locales.entries()) {
  if(r.batch&&(i<r.batch.start||i>=r.batch.end))continue;
  const cal=r.calendar(locale.theme,0,i*504,{name:'Language · '+locale.code+' · calendar'});
  const previous=penpot.root.children.find(b=>b.name==='Language · '+locale.code+' · settings');
  if(previous)previous.name='Windows · General · language-'+locale.code+' · '+locale.theme;
  const settings=r.settingsContent('General',locale.theme,416,i*504,'Windows','language-'+locale.code);
  const conflict=r.settingsContent('General',locale.theme,928,i*504,'Windows','conflict-language-'+locale.code);
  const additional={ru:{Calendar:'Календарь',Accounts:'Аккаунты',App:'Приложение','Not set':'Не задано','Record…':'Задать…',Updates:'Обновления','Check now':'Проверить','Tray icon':'Значок в трее',Color:'Цвет',Mono:'Моно','About Lightcal':'О Lightcal'},es:{Calendar:'Calendario',Accounts:'Cuentas',App:'Aplicación','Not set':'Sin asignar','Record…':'Definir…',Updates:'Actualizaciones','Check now':'Comprobar','Tray icon':'Icono',Color:'Color',Mono:'Mono','About Lightcal':'Acerca de Lightcal'},ja:{Calendar:'カレンダー',Accounts:'アカウント',App:'アプリ','Not set':'未設定','Record…':'設定…',Updates:'更新','Check now':'確認','Tray icon':'トレイアイコン',Color:'カラー',Mono:'単色','About Lightcal':'Lightcalについて','Keyboard shortcut':'ショートカット'}};
  Object.assign(locale.map,additional[locale.code]||{});
  Object.assign(locale.map,{ru:{'Used by another app':'Занято другим приложением'},es:{'Used by another app':'Lo usa otra aplicación'},ja:{'Used by another app':'他のアプリが使用中'}}[locale.code]||{});
  for(const b of [cal,settings,conflict]) {
    b.name='Language · '+locale.code+' · '+(b===cal?'calendar':b===settings?'settings':'shortcut conflict');
    for(const s of penpotUtils.findShapes(s=>s.type==='text',b)) {
      if(locale.map[s.characters])s.characters=locale.map[s.characters];
      if(locale.code==='ja'&&/[\u3040-\u30FF\u3400-\u9FFF]/.test(s.characters)) {
        const font=penpot.fonts.all.find(f=>f.name==='Noto Sans JP'),weight=s.getPluginData('lightcal-font-weight')||s.fontWeight,size=s.fontSize;
        if(font){font.applyToText(s,font.variants.find(v=>v.fontWeight===weight&&v.fontStyle!=='italic'));s.fontWeight=weight;s.fontSize=size;}
      }
    }
  }
}
return {page:r.currentPage.name,boards:r.currentPage.root.children.filter(s=>s.type==='board').length};
