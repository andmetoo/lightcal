//! Bundled message catalogs and locale-aware date/plural formatting for `en`, `ru`, `es`, and `ja`.

use crate::preferences::LocaleCode;
use calendar_core::{LocalDate, Weekday, YearMonth};

/// Complete connection/update control labels; provider diagnostics fall back to English.
pub fn connection_labels(locale: LocaleCode) -> [&'static str; 11] {
    match locale {
        LocaleCode::En => [
            "Microsoft Desktop Client ID",
            "Connect Outlook",
            "Disconnect Outlook",
            "ICS calendar name",
            "Private HTTPS ICS address",
            "Add ICS",
            "Remove",
            "Windows calendars (legacy, optional)",
            "Install and restart",
            "Automatically install signed updates",
            "Read-only Outlook and private ICS subscriptions",
        ],
        LocaleCode::Ru => [
            "Client ID приложения Microsoft",
            "Войти в Outlook",
            "Отключить Outlook",
            "Название календаря ICS",
            "Секретная HTTPS-ссылка ICS",
            "Добавить ICS",
            "Удалить",
            "Календари Windows (устаревший способ)",
            "Установить",
            "Автоматически устанавливать подписанные обновления",
            "Календари Outlook и ICS — только чтение",
        ],
        LocaleCode::Es => [
            "Client ID de Microsoft para escritorio",
            "Conectar Outlook",
            "Desconectar Outlook",
            "Nombre del calendario ICS",
            "Dirección HTTPS privada de ICS",
            "Añadir ICS",
            "Eliminar",
            "Calendarios de Windows (opcional)",
            "Instalar y reiniciar",
            "Instalar automáticamente actualizaciones firmadas",
            "Outlook e ICS privados, solo lectura",
        ],
        LocaleCode::Ja => [
            "Microsoft デスクトップ Client ID",
            "Outlook に接続",
            "Outlook を切断",
            "ICS カレンダー名",
            "非公開 HTTPS ICS アドレス",
            "ICS を追加",
            "削除",
            "Windows カレンダー（旧方式・任意）",
            "インストールして再起動",
            "署名済み更新を自動インストール",
            "Outlook と非公開 ICS を読み取り専用で表示",
        ],
    }
}

#[derive(Debug, Clone)]
pub struct UiStrings {
    pub app_name: &'static str,
    pub current_month_badge: &'static str,
    pub today_button: &'static str,
    pub week_col_header: &'static str,
    pub selected_date_overline: &'static str,
    pub settings_title: &'static str,
    pub nav_general: &'static str,
    pub nav_appearance: &'static str,
    pub nav_calendars: &'static str,
    pub nav_about: &'static str,
    pub general_subtitle: &'static str,
    pub label_language: &'static str,
    pub desc_language: &'static str,
    pub label_startup: &'static str,
    pub desc_startup: &'static str,
    pub label_shortcut: &'static str,
    pub desc_shortcut: &'static str,
    pub shortcut_unset: &'static str,
    pub shortcut_record: &'static str,
    pub shortcut_clear: &'static str,
    pub appearance_subtitle: &'static str,
    pub label_theme: &'static str,
    pub desc_theme: &'static str,
    pub theme_system: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub label_first_weekday: &'static str,
    pub desc_first_weekday: &'static str,
    pub label_gray_weekdays: &'static str,
    pub desc_gray_weekdays: &'static str,
    pub label_show_weekday: &'static str,
    pub desc_show_weekday: &'static str,
    pub label_show_week_numbers: &'static str,
    pub desc_show_week_numbers: &'static str,
    pub calendars_subtitle: &'static str,
    pub provider_heading: &'static str,
    pub provider_readonly_tag: &'static str,
    pub refresh_button: &'static str,
    pub calendars_empty_title: &'static str,
    pub calendars_denied_title: &'static str,
    pub calendars_unavailable_desc: &'static str,
    pub calendars_dot_note: &'static str,
    pub about_subtitle: &'static str,
    pub about_license_line: &'static str,
    pub updates_heading: &'static str,
    pub updates_idle_desc: &'static str,
    pub updates_checking_desc: &'static str,
    pub updates_check_button: &'static str,
    pub updates_open_release_button: &'static str,
    pub tray_open_calendar: &'static str,
    pub tray_settings: &'static str,
    pub tray_about: &'static str,
    pub tray_quit: &'static str,
    pub back_to_calendar: &'static str,
    pub label_tray_mode: &'static str,
    pub desc_tray_mode: &'static str,
    pub label_tray_palette: &'static str,
    pub desc_tray_palette: &'static str,
    pub palette_colored: &'static str,
    pub palette_monochrome: &'static str,
    // Redesigned settings strings
    pub preferences_subtitle: &'static str,
    pub lang_en_us: &'static str,
    pub lang_ru: &'static str,
    pub lang_es: &'static str,
    pub lang_ja: &'static str,
    pub start_of_week_monday: &'static str,
    pub start_of_week_sunday: &'static str,
    pub shortcut_conflict_title: &'static str,
    pub shortcut_conflict_hint: &'static str,
    pub shortcut_reset_button: &'static str,
    pub shortcut_conflict_badge: &'static str,
    pub theme_light_obsidian: &'static str,
    pub theme_dark_obsidian: &'static str,
    pub theme_system_obsidian: &'static str,
    pub label_highlight_current_month: &'static str,
    pub desc_highlight_current_month: &'static str,
    pub label_show_event_dots: &'static str,
    pub desc_show_event_dots: &'static str,
    pub add_calendar_button: &'static str,
    pub add_calendar_account_button: &'static str,
    pub open_privacy_settings_button: &'static str,
    pub access_denied_desc: &'static str,
    pub retry_sync_button: &'static str,
    pub app_version_line: &'static str,
    pub checking_updates_label: &'static str,
    pub update_available_badge: &'static str,
    pub update_ready_title: &'static str,
    pub update_ready_notes: &'static str,
    pub release_notes_button: &'static str,
    pub label_auto_updates: &'static str,
    pub desc_auto_updates: &'static str,
    pub label_update_channel: &'static str,
    pub desc_update_channel: &'static str,
    pub channel_stable: &'static str,
    pub channel_preview: &'static str,
    pub label_license_source: &'static str,
    pub desc_license_source: &'static str,
    pub github_repo_button: &'static str,
    pub koona_app_button: &'static str,
    pub outlook_section: &'static str,
    pub ics_section: &'static str,
    pub ics_help: &'static str,
    pub startup_error: &'static str,
}

const EN_STRINGS: UiStrings = UiStrings {
    app_name: "Lightcal",
    current_month_badge: "Current month",
    today_button: "Today",
    week_col_header: "W",
    selected_date_overline: "SELECTED DATE",
    settings_title: "Settings",
    nav_general: "General",
    nav_appearance: "Appearance",
    nav_calendars: "Calendars",
    nav_about: "About",
    general_subtitle: "Regional conventions, startup behavior, and global keyboard shortcuts.",
    label_language: "Language",
    desc_language: "Interface language for month names, weekdays, and controls",
    label_startup: "Launch at login",
    desc_startup: "Start Lightcal quietly in the system tray when you sign in",
    label_shortcut: "Global shortcut",
    desc_shortcut: "Toggle the calendar popover from anywhere on the desktop",
    shortcut_unset: "Not set",
    shortcut_record: "Record",
    shortcut_clear: "Clear",
    appearance_subtitle: "Theme, gray weekdays, event dots, and the tray icon.",
    label_theme: "Theme",
    desc_theme: "Choose Alabaster Light, Obsidian Dark, or follow system appearance",
    theme_system: "System default",
    theme_light: "Light (Alabaster)",
    theme_dark: "Dark (Obsidian)",
    label_first_weekday: "Start of week",
    desc_first_weekday: "First column shown in the calendar grid",
    label_gray_weekdays: "Gray days of week",
    desc_gray_weekdays: "Tint non-working weekday columns with a soft vertical band",
    label_show_weekday: "Show weekday in dates",
    desc_show_weekday: "Include weekday name in today and selected-date text.",
    label_show_week_numbers: "Show week numbers",
    desc_show_week_numbers: "Display ISO week numbers (W01–W52) along the left margin",
    calendars_subtitle: "Read-only Outlook and private ICS links. Checked calendars show event dots.",
    provider_heading: "Windows Calendar Store",
    provider_readonly_tag: "Read-only access",
    refresh_button: "Refresh",
    calendars_empty_title: "No calendar accounts connected yet",
    calendars_denied_title: "Calendar access is turned off in System Privacy Settings",
    calendars_unavailable_desc: "Nothing is connected yet. Add Outlook or a private ICS link, then choose which calendars to show.",
    calendars_dot_note: "Only checked calendars contribute event dots (up to 3 dots per day).",
    about_subtitle: "Version status, release channel, and automatic update preferences.",
    about_license_line: "GPL-3.0-only · Open source",
    updates_heading: "Updates",
    updates_idle_desc: "Checks GitHub Releases only when you ask. Never auto-installs.",
    updates_checking_desc: "Checking GitHub for the latest stable release…",
    updates_check_button: "Check for updates",
    updates_open_release_button: "Open release page ↗",
    tray_open_calendar: "Open calendar",
    tray_settings: "Settings",
    tray_about: "About Lightcal",
    tray_quit: "Quit",
    back_to_calendar: "← Back to Calendar",
    label_tray_mode: "Tray / Status Bar icon mode",
    desc_tray_mode: "Date, weekday and date, or time. This changes Lightcal's icon only.",
    label_tray_palette: "Tray / Status Bar icon palette",
    desc_tray_palette: "Colored Coral (#E5484D) cap or native OS Monochrome",
    palette_colored: "Colored (#E5484D)",
    palette_monochrome: "Monochrome",
    preferences_subtitle: "Preferences",
    lang_en_us: "English (US)",
    lang_ru: "Русский",
    lang_es: "Español",
    lang_ja: "日本語",
    start_of_week_monday: "Monday",
    start_of_week_sunday: "Sunday",
    shortcut_conflict_title: "Shortcut unavailable: Alt + Space is already registered by Raycast.",
    shortcut_conflict_hint: "Press a new key combination (for example Alt + C) or click Reset.",
    shortcut_reset_button: "Reset",
    shortcut_conflict_badge: "Conflict",
    theme_light_obsidian: "Light (Alabaster)",
    theme_dark_obsidian: "Dark (Obsidian)",
    theme_system_obsidian: "System default",
    label_highlight_current_month: "Highlight current month",
    desc_highlight_current_month:
        "Show Indigo underline and status pill when viewing the active month",
    label_show_event_dots: "Show event dots on dates",
    desc_show_event_dots: "Render up to 3 semantic color dots beneath days with scheduled events",
    add_calendar_button: "+ Add Calendar",
    add_calendar_account_button: "+ Add Calendar Account",
    open_privacy_settings_button: "Open Privacy Settings",
    access_denied_desc:
        "Lightcal needs permission to read calendar events. The month view still works without them.",
    retry_sync_button: "Retry Sync",
    app_version_line: "Lightcal · Rust + Slint",
    checking_updates_label: "Checking updates…",
    update_available_badge: "Update available",
    update_ready_title: "A newer version is available",
    update_ready_notes: "Open the release page to review it. You choose when to install.",
    release_notes_button: "Release notes",
    label_auto_updates: "Automatic updates",
    desc_auto_updates: "Download and verify signed releases in the background",
    label_update_channel: "Update channel",
    desc_update_channel: "Receive verified stable releases or preview builds",
    channel_stable: "Stable",
    channel_preview: "Preview",
    label_license_source: "License & source",
    desc_license_source: "GPL-3.0-only · Built with Rust, Slint, and zero webview bloat",
    github_repo_button: "GitHub ↗",
    koona_app_button: "koona.app ↗",
    outlook_section: "Outlook",
    ics_section: "Private calendar link",
    ics_help: "Paste a secret HTTPS ICS address. It stays on this computer and is hidden after you type it.",
    startup_error: "Could not update the Windows startup entry.",
};

const RU_STRINGS: UiStrings = UiStrings {
    app_name: "Lightcal",
    current_month_badge: "Текущий месяц",
    today_button: "Сегодня",
    week_col_header: "НЕД",
    selected_date_overline: "ВЫБРАННАЯ ДАТА",
    settings_title: "Настройки",
    nav_general: "Общие",
    nav_appearance: "Вид",
    nav_calendars: "Календари",
    nav_about: "О программе",
    general_subtitle: "Язык интерфейса, автозапуск и глобальное сочетание клавиш.",
    label_language: "Язык интерфейса",
    desc_language: "Мгновенно обновляет интерфейс, дни недели и формат дат.",
    label_startup: "Запускать вместе с Windows",
    desc_startup: "Открывать свернутым в системном лотке при входе в систему.",
    label_shortcut: "Глобальное сочетание клавиш",
    desc_shortcut: "Открывать или скрывать компактный календарь из любого окна.",
    shortcut_unset: "Не задано",
    shortcut_record: "Записать",
    shortcut_clear: "Сбросить",
    appearance_subtitle: "Тема оформления, первый день недели и выделенные колонки.",
    label_theme: "Тема оформления",
    desc_theme: "Системная, светлая или тёмная тема.",
    theme_system: "Системная",
    theme_light: "Светлая",
    theme_dark: "Тёмная",
    label_first_weekday: "Первый день недели",
    desc_first_weekday: "Настраивается независимо от языка интерфейса.",
    label_gray_weekdays: "Выделенные дни недели",
    desc_gray_weekdays: "Мягкий нейтральный фон для выбранных колонок дней недели.",
    label_show_weekday: "Показывать день недели",
    desc_show_weekday: "Отображать день недели в сегодняшней и выбранной дате.",
    label_show_week_numbers: "Номера недель ISO",
    desc_show_week_numbers: "Показывать колонку номеров недель слева от сетки месяца.",
    calendars_subtitle: "Календари Windows (только чтение) для точек событий.",
    provider_heading: "Календари Windows",
    provider_readonly_tag: "Только чтение",
    refresh_button: "Обновить",
    calendars_empty_title: "Календари не найдены",
    calendars_denied_title: "Доступ к календарям отключён в параметрах конфиденциальности",
    calendars_unavailable_desc: "Календарь продолжает работать автономно без точек событий.",
    calendars_dot_note: "Точки событий показываются только для отмеченных календарей (до 3 точек).",
    about_subtitle: "Сведения о версии, ссылки проекта и ручная проверка обновлений.",
    about_license_line: "Лицензия GPL-3.0-only · Открытый исходный код",
    updates_heading: "Обновления",
    updates_idle_desc: "Проверяет релизы на GitHub только по вашему запросу.",
    updates_checking_desc: "Проверка последнего стабильного релиза на GitHub…",
    updates_check_button: "Проверить обновления",
    updates_open_release_button: "Открыть релиз на GitHub",
    tray_open_calendar: "Открыть календарь",
    tray_settings: "Настройки",
    tray_about: "О программе Lightcal",
    tray_quit: "Выход",
    back_to_calendar: "← К календарю",
    label_tray_mode: "Режим иконки в трее и статусбаре",
    desc_tray_mode: "Дата, день недели и дата, или время. Меняется только значок Lightcal.",
    label_tray_palette: "Палитра иконки в трее",
    desc_tray_palette: "Цветная коралловая шапка (#E5484D) или системная монохромная",
    palette_colored: "Цветная (#E5484D)",
    palette_monochrome: "Монохромная",
    preferences_subtitle: "Параметры",
    lang_en_us: "English (US)",
    lang_ru: "Русский",
    lang_es: "Español",
    lang_ja: "日本語",
    start_of_week_monday: "Понедельник",
    start_of_week_sunday: "Воскресенье",
    shortcut_conflict_title: "Сочетание недоступно: комбинация уже занята другим приложением.",
    shortcut_conflict_hint: "Нажмите другое сочетание клавиш или нажмите «Сбросить».",
    shortcut_reset_button: "Сбросить",
    shortcut_conflict_badge: "Конфликт",
    theme_light_obsidian: "Светлая (Alabaster)",
    theme_dark_obsidian: "Тёмная (Obsidian)",
    theme_system_obsidian: "Как в системе",
    label_highlight_current_month: "Выделять текущий месяц",
    desc_highlight_current_month: "Индиго-подчёркивание и индикатор при просмотре текущего месяца",
    label_show_event_dots: "Точки событий в датах",
    desc_show_event_dots: "До 3 цветных точек под днями с запланированными событиями",
    add_calendar_button: "+ Добавить календарь",
    add_calendar_account_button: "+ Добавить учётную запись",
    open_privacy_settings_button: "Параметры конфиденциальности",
    access_denied_desc:
        "Lightcal требуются разрешения на чтение для отображения локальных событий.",
    retry_sync_button: "Повторить",
    app_version_line: "Lightcal · Rust + Slint",
    checking_updates_label: "Проверка обновлений…",
    update_available_badge: "Доступно обновление",
    update_ready_title: "Доступна новая версия",
    update_ready_notes: "Откройте страницу релиза, чтобы посмотреть изменения. Установка только по вашему выбору.",
    release_notes_button: "Что нового ↗",
    label_auto_updates: "Автоматические обновления",
    desc_auto_updates: "Фоновая загрузка и проверка подписанных релизов",
    label_update_channel: "Канал обновлений",
    desc_update_channel: "Получение стабильных релизов или предварительных версий",
    channel_stable: "Стабильный",
    channel_preview: "Предварительный",
    label_license_source: "Лицензия и исходный код",
    desc_license_source: "Лицензия GPL-3.0-only · Rust + Slint без лишнего webview",
    github_repo_button: "GitHub ↗",
    koona_app_button: "koona.app ↗",
    outlook_section: "Outlook",
    ics_section: "Ссылка на личный календарь",
    ics_help: "Вставьте секретную HTTPS-ссылку ICS. Она хранится только на этом компьютере и скрывается после ввода.",
    startup_error: "Не удалось изменить автозапуск Windows.",
};

const ES_STRINGS: UiStrings = UiStrings {
    app_name: "Lightcal",
    current_month_badge: "Mes actual",
    today_button: "Hoy",
    week_col_header: "SEM",
    selected_date_overline: "FECHA SELECCIONADA",
    settings_title: "Ajustes",
    nav_general: "General",
    nav_appearance: "Apariencia",
    nav_calendars: "Calendarios",
    nav_about: "Acerca de",
    general_subtitle: "Idioma, inicio con Windows y atajo de teclado global.",
    label_language: "Idioma",
    desc_language: "Actualiza textos, días de la semana y fechas al instante.",
    label_startup: "Iniciar con Windows",
    desc_startup: "Abrir silenciosamente en la bandeja al iniciar sesión.",
    label_shortcut: "Atajo global",
    desc_shortcut: "Mostrar u ocultar el calendario compacto desde cualquier aplicación.",
    shortcut_unset: "Sin asignar",
    shortcut_record: "Grabar",
    shortcut_clear: "Borrar",
    appearance_subtitle: "Tema visual, primer día de la semana y columnas sombreadas.",
    label_theme: "Tema",
    desc_theme: "Usar el tema del sistema o elegir Claro / Oscuro.",
    theme_system: "Sistema",
    theme_light: "Claro",
    theme_dark: "Oscuro",
    label_first_weekday: "Primer día de la semana",
    desc_first_weekday: "Independiente del idioma de la interfaz.",
    label_gray_weekdays: "Días sombreados",
    desc_gray_weekdays: "Aplicar un fondo neutro suave a cualquier subconjunto de días.",
    label_show_weekday: "Mostrar día de la semana",
    desc_show_weekday: "Incluir el día de la semana en la fecha actual y seleccionada.",
    label_show_week_numbers: "Números de semana ISO",
    desc_show_week_numbers: "Mostrar la columna de semanas ISO junto a la cuadrícula.",
    calendars_subtitle: "Calendarios de Windows de solo lectura para puntos de eventos.",
    provider_heading: "Calendarios de Windows",
    provider_readonly_tag: "Solo lectura",
    refresh_button: "Actualizar",
    calendars_empty_title: "No se encontraron calendarios",
    calendars_denied_title: "El acceso al calendario no está disponible",
    calendars_unavailable_desc: "Lightcal sigue funcionando sin conexión. No se muestran eventos.",
    calendars_dot_note:
        "Solo los calendarios marcados muestran puntos de eventos (máx. 3 por día).",
    about_subtitle: "Versión, enlaces de código abierto y búsqueda manual de actualizaciones.",
    about_license_line: "GPL-3.0-only · Código abierto",
    updates_heading: "Actualizaciones",
    updates_idle_desc: "Comprueba lanzamientos en GitHub solo cuando lo solicitas.",
    updates_checking_desc: "Buscando la última versión estable en GitHub…",
    updates_check_button: "Buscar actualizaciones",
    updates_open_release_button: "Ver versión en GitHub",
    tray_open_calendar: "Abrir calendario",
    tray_settings: "Ajustes",
    tray_about: "Acerca de Lightcal",
    tray_quit: "Salir",
    back_to_calendar: "← Volver al calendario",
    label_tray_mode: "Modo del icono de bandeja",
    desc_tray_mode: "Fecha, día y fecha, o la hora. Solo cambia el icono de Lightcal.",
    label_tray_palette: "Paleta del icono de bandeja",
    desc_tray_palette: "Cabezal coral (#E5484D) o monocromo del sistema",
    palette_colored: "Color (#E5484D)",
    palette_monochrome: "Monocromo",
    preferences_subtitle: "Preferencias",
    lang_en_us: "English (US)",
    lang_ru: "Русский",
    lang_es: "Español",
    lang_ja: "日本語",
    start_of_week_monday: "Lunes",
    start_of_week_sunday: "Domingo",
    shortcut_conflict_title: "Atajo no disponible: combinación ya registrada por otra aplicación.",
    shortcut_conflict_hint: "Presione una nueva combinación o haga clic en Restablecer.",
    shortcut_reset_button: "Restablecer",
    shortcut_conflict_badge: "Conflicto",
    theme_light_obsidian: "Claro (Alabaster)",
    theme_dark_obsidian: "Oscuro (Obsidian)",
    theme_system_obsidian: "Predeterminado del sistema",
    label_highlight_current_month: "Destacar mes actual",
    desc_highlight_current_month: "Subrayado en índigo e indicador de estado en el mes activo",
    label_show_event_dots: "Puntos de eventos en fechas",
    desc_show_event_dots: "Muestra hasta 3 puntos de color en días con eventos programados",
    add_calendar_button: "+ Añadir calendario",
    add_calendar_account_button: "+ Añadir cuenta de calendario",
    open_privacy_settings_button: "Configuración de privacidad",
    access_denied_desc: "Lightcal necesita permiso de lectura para mostrar eventos locales.",
    retry_sync_button: "Reintentar sincronización",
    app_version_line: "Lightcal · Rust + Slint",
    checking_updates_label: "Buscando actualizaciones…",
    update_available_badge: "Actualización disponible",
    update_ready_title: "Hay una versión más reciente",
    update_ready_notes:
        "Abra la página de la versión para revisarla. Usted decide cuándo instalar.",
    release_notes_button: "Notas de la versión ↗",
    label_auto_updates: "Actualizaciones automáticas",
    desc_auto_updates: "Descargar y verificar versiones firmadas en segundo plano",
    label_update_channel: "Canal de actualizaciones",
    desc_update_channel: "Recibir versiones estables o compilaciones preliminares",
    channel_stable: "Estable",
    channel_preview: "Vista previa",
    label_license_source: "Licencia y código fuente",
    desc_license_source: "Licencia GPL-3.0-only · Creado con Rust y Slint sin webview",
    github_repo_button: "GitHub ↗",
    koona_app_button: "koona.app ↗",
    outlook_section: "Outlook",
    ics_section: "Enlace de calendario privado",
    ics_help:
        "Pegue una dirección HTTPS ICS secreta. Permanece en este equipo y se oculta al escribirla.",
    startup_error: "No se pudo cambiar el inicio con Windows.",
};

const JA_STRINGS: UiStrings = UiStrings {
    app_name: "Lightcal",
    current_month_badge: "今月",
    today_button: "今日",
    week_col_header: "週",
    selected_date_overline: "選択した日付",
    settings_title: "設定",
    nav_general: "一般",
    nav_appearance: "外観",
    nav_calendars: "カレンダー",
    nav_about: "Lightcal について",
    general_subtitle: "表示言語、Windows 起動時の自動開始、グローバルショートカット。",
    label_language: "表示言語",
    desc_language: "再起動せずに UI と日付の表記をすぐに切り替えます。",
    label_startup: "Windows 起動時に開始",
    desc_startup: "サインイン時にシステムトレイへ最小化した状態で起動します。",
    label_shortcut: "グローバルショートカット",
    desc_shortcut: "どのアプリからでもトレイカレンダーを表示・非表示にします。",
    shortcut_unset: "未設定",
    shortcut_record: "登録",
    shortcut_clear: "クリア",
    appearance_subtitle: "テーマ、週の開始曜日、グレー表示する曜日の選択。",
    label_theme: "テーマ",
    desc_theme: "システム設定に従うか、ライト・ダークを固定します。",
    theme_system: "システム",
    theme_light: "ライト",
    theme_dark: "ダーク",
    label_first_weekday: "週の最初の曜日",
    desc_first_weekday: "表示言語とは独立して設定できます。",
    label_gray_weekdays: "グレー表示する曜日",
    desc_gray_weekdays: "任意の曜日の列に落ち着いた背景色を適用します。",
    label_show_weekday: "日付に曜日を表示",
    desc_show_weekday: "今日と選択した日付に曜日を含めます。",
    label_show_week_numbers: "ISO 週番号を表示",
    desc_show_week_numbers: "月グリッドの左側に ISO 週番号を表示します。",
    calendars_subtitle: "予定ドットを表示する Windows カレンダー（読み取り専用）を選択します。",
    provider_heading: "Windows カレンダー",
    provider_readonly_tag: "読み取り専用",
    refresh_button: "更新",
    calendars_empty_title: "カレンダーが見つかりません",
    calendars_denied_title: "カレンダーへのアクセスが利用できません",
    calendars_unavailable_desc: "予定は表示されませんが、カレンダー機能はそのまま利用できます。",
    calendars_dot_note: "チェックしたカレンダーの予定のみドット表示されます（最大3個）。",
    about_subtitle: "バージョン情報、リンク、手動アップデート確認。",
    about_license_line: "GPL-3.0-only · オープンソース",
    updates_heading: "アップデート",
    updates_idle_desc: "ボタンを押したときのみ GitHub の安定版リリースを確認します。",
    updates_checking_desc: "GitHub で最新の安定版リリースを確認しています…",
    updates_check_button: "アップデートを確認",
    updates_open_release_button: "GitHub でリリースを見る",
    tray_open_calendar: "カレンダーを開く",
    tray_settings: "設定",
    tray_about: "Lightcal について",
    tray_quit: "終了",
    back_to_calendar: "← カレンダーへ戻る",
    label_tray_mode: "トレイ／メニューバーアイコンの表示",
    desc_tray_mode: "日付、曜日と日付、または時刻。変わるのは Lightcal のアイコンだけです。",
    label_tray_palette: "アイコンのカラーパレット",
    desc_tray_palette: "コーラル (#E5484D) またはシステムのモノクロ表示",
    palette_colored: "カラー (#E5484D)",
    palette_monochrome: "モノクロ",
    preferences_subtitle: "環境設定",
    lang_en_us: "English (US)",
    lang_ru: "Русский",
    lang_es: "Español",
    lang_ja: "日本語",
    start_of_week_monday: "月曜日",
    start_of_week_sunday: "日曜日",
    shortcut_conflict_title: "ショートカットを使用できません: 他のアプリで既に使用されています。",
    shortcut_conflict_hint:
        "別のキーの組み合わせを入力するか、「リセット」をクリックしてください。",
    shortcut_reset_button: "リセット",
    shortcut_conflict_badge: "競合",
    theme_light_obsidian: "ライト (Alabaster)",
    theme_dark_obsidian: "ダーク (Obsidian)",
    theme_system_obsidian: "システム設定",
    label_highlight_current_month: "今月の強調表示",
    desc_highlight_current_month: "現在の月を表示中にインディゴの下線と状態バッジを表示します",
    label_show_event_dots: "日付に予定ドットを表示",
    desc_show_event_dots: "予定のある日付の下に最大3個のカラー印を表示します",
    add_calendar_button: "+ カレンダーを追加",
    add_calendar_account_button: "+ カレンダーアカウントを追加",
    open_privacy_settings_button: "プライバシー設定を開く",
    access_denied_desc: "Lightcal が予定を表示するにはカレンダーの読み取り権限が必要です。",
    retry_sync_button: "再同期",
    app_version_line: "Lightcal · Rust + Slint",
    checking_updates_label: "アップデートを確認中…",
    update_available_badge: "アップデートがあります",
    update_ready_title: "新しいバージョンがあります",
    update_ready_notes: "リリースページで内容を確認できます。インストールするタイミングは自分で選べます。",
    release_notes_button: "リリースノート ↗",
    label_auto_updates: "自動アップデート",
    desc_auto_updates: "署名付きリリースをバックグラウンドでダウンロードおよび検証します",
    label_update_channel: "更新チャンネル",
    desc_update_channel: "安定版またはプレビュー版の受信を選択します",
    channel_stable: "安定版",
    channel_preview: "プレビュー版",
    label_license_source: "ライセンスとソースコード",
    desc_license_source: "GPL-3.0-only · Rust と Slint で構築、ウェブビュー不使用",
    github_repo_button: "GitHub ↗",
    koona_app_button: "koona.app ↗",
    outlook_section: "Outlook",
    ics_section: "非公開カレンダーのリンク",
    ics_help: "秘密の HTTPS ICS アドレスを貼り付けます。このパソコンにだけ保存され、入力後は表示されません。",
    startup_error: "Windows のスタートアップを変更できませんでした。",
};

pub fn catalog(locale: LocaleCode) -> &'static UiStrings {
    match locale {
        LocaleCode::En => &EN_STRINGS,
        LocaleCode::Ru => &RU_STRINGS,
        LocaleCode::Es => &ES_STRINGS,
        LocaleCode::Ja => &JA_STRINGS,
    }
}

pub fn weekday_short(locale: LocaleCode, day: Weekday) -> &'static str {
    match (locale, day) {
        (LocaleCode::En, Weekday::Monday) => "Mo",
        (LocaleCode::En, Weekday::Tuesday) => "Tu",
        (LocaleCode::En, Weekday::Wednesday) => "We",
        (LocaleCode::En, Weekday::Thursday) => "Th",
        (LocaleCode::En, Weekday::Friday) => "Fr",
        (LocaleCode::En, Weekday::Saturday) => "Sa",
        (LocaleCode::En, Weekday::Sunday) => "Su",

        (LocaleCode::Ru, Weekday::Monday) => "Пн",
        (LocaleCode::Ru, Weekday::Tuesday) => "Вт",
        (LocaleCode::Ru, Weekday::Wednesday) => "Ср",
        (LocaleCode::Ru, Weekday::Thursday) => "Чт",
        (LocaleCode::Ru, Weekday::Friday) => "Пт",
        (LocaleCode::Ru, Weekday::Saturday) => "Сб",
        (LocaleCode::Ru, Weekday::Sunday) => "Вс",

        (LocaleCode::Es, Weekday::Monday) => "Lu",
        (LocaleCode::Es, Weekday::Tuesday) => "Ma",
        (LocaleCode::Es, Weekday::Wednesday) => "Mi",
        (LocaleCode::Es, Weekday::Thursday) => "Ju",
        (LocaleCode::Es, Weekday::Friday) => "Vi",
        (LocaleCode::Es, Weekday::Saturday) => "Sá",
        (LocaleCode::Es, Weekday::Sunday) => "Do",

        (LocaleCode::Ja, Weekday::Monday) => "月",
        (LocaleCode::Ja, Weekday::Tuesday) => "火",
        (LocaleCode::Ja, Weekday::Wednesday) => "水",
        (LocaleCode::Ja, Weekday::Thursday) => "木",
        (LocaleCode::Ja, Weekday::Friday) => "金",
        (LocaleCode::Ja, Weekday::Saturday) => "土",
        (LocaleCode::Ja, Weekday::Sunday) => "日",
    }
}

pub fn weekday_full(locale: LocaleCode, day: Weekday) -> &'static str {
    match (locale, day) {
        (LocaleCode::En, Weekday::Monday) => "Monday",
        (LocaleCode::En, Weekday::Tuesday) => "Tuesday",
        (LocaleCode::En, Weekday::Wednesday) => "Wednesday",
        (LocaleCode::En, Weekday::Thursday) => "Thursday",
        (LocaleCode::En, Weekday::Friday) => "Friday",
        (LocaleCode::En, Weekday::Saturday) => "Saturday",
        (LocaleCode::En, Weekday::Sunday) => "Sunday",

        (LocaleCode::Ru, Weekday::Monday) => "Понедельник",
        (LocaleCode::Ru, Weekday::Tuesday) => "Вторник",
        (LocaleCode::Ru, Weekday::Wednesday) => "Среда",
        (LocaleCode::Ru, Weekday::Thursday) => "Четверг",
        (LocaleCode::Ru, Weekday::Friday) => "Пятница",
        (LocaleCode::Ru, Weekday::Saturday) => "Суббота",
        (LocaleCode::Ru, Weekday::Sunday) => "Воскресенье",

        (LocaleCode::Es, Weekday::Monday) => "Lunes",
        (LocaleCode::Es, Weekday::Tuesday) => "Martes",
        (LocaleCode::Es, Weekday::Wednesday) => "Miércoles",
        (LocaleCode::Es, Weekday::Thursday) => "Jueves",
        (LocaleCode::Es, Weekday::Friday) => "Viernes",
        (LocaleCode::Es, Weekday::Saturday) => "Sábado",
        (LocaleCode::Es, Weekday::Sunday) => "Domingo",

        (LocaleCode::Ja, Weekday::Monday) => "月曜日",
        (LocaleCode::Ja, Weekday::Tuesday) => "火曜日",
        (LocaleCode::Ja, Weekday::Wednesday) => "水曜日",
        (LocaleCode::Ja, Weekday::Thursday) => "木曜日",
        (LocaleCode::Ja, Weekday::Friday) => "金曜日",
        (LocaleCode::Ja, Weekday::Saturday) => "土曜日",
        (LocaleCode::Ja, Weekday::Sunday) => "日曜日",
    }
}

pub fn month_name(locale: LocaleCode, month: u8) -> &'static str {
    let idx = (month.clamp(1, 12) - 1) as usize;
    const EN: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const RU: [&str; 12] = [
        "Январь",
        "Февраль",
        "Март",
        "Апрель",
        "Май",
        "Июнь",
        "Июль",
        "Август",
        "Сентябрь",
        "Октябрь",
        "Ноябрь",
        "Декабрь",
    ];
    const ES: [&str; 12] = [
        "Enero",
        "Febrero",
        "Marzo",
        "Abril",
        "Mayo",
        "Junio",
        "Julio",
        "Agosto",
        "Septiembre",
        "Octubre",
        "Noviembre",
        "Diciembre",
    ];
    const JA: [&str; 12] = [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ];
    match locale {
        LocaleCode::En => EN[idx],
        LocaleCode::Ru => RU[idx],
        LocaleCode::Es => ES[idx],
        LocaleCode::Ja => JA[idx],
    }
}

fn month_genitive_ru(month: u8) -> &'static str {
    const RU_GEN: [&str; 12] = [
        "января",
        "февраля",
        "марта",
        "апреля",
        "мая",
        "июня",
        "июля",
        "августа",
        "сентября",
        "октября",
        "ноября",
        "декабря",
    ];
    RU_GEN[(month.clamp(1, 12) - 1) as usize]
}

pub fn format_month_year(locale: LocaleCode, ym: YearMonth) -> String {
    match locale {
        LocaleCode::Ja => format!("{}年{}", ym.year, month_name(locale, ym.month)),
        LocaleCode::Es => format!("{} {}", month_name(locale, ym.month), ym.year),
        _ => format!("{} {}", month_name(locale, ym.month), ym.year),
    }
}

pub fn format_date_line(locale: LocaleCode, date: LocalDate, include_weekday: bool) -> String {
    let wd = weekday_full(locale, date.weekday());
    match (locale, include_weekday) {
        (LocaleCode::En, true) => format!("{wd}, {} {}", date.day, month_name(locale, date.month)),
        (LocaleCode::En, false) => format!(
            "{} {}, {}",
            month_name(locale, date.month),
            date.day,
            date.year
        ),
        (LocaleCode::Ru, true) => format!("{wd}, {} {}", date.day, month_genitive_ru(date.month)),
        (LocaleCode::Ru, false) => format!(
            "{} {} {}",
            date.day,
            month_genitive_ru(date.month),
            date.year
        ),
        (LocaleCode::Es, true) => format!(
            "{wd}, {} de {}",
            date.day,
            month_name(locale, date.month).to_lowercase()
        ),
        (LocaleCode::Es, false) => format!(
            "{} de {} de {}",
            date.day,
            month_name(locale, date.month).to_lowercase(),
            date.year
        ),
        (LocaleCode::Ja, true) => format!("{}月{}日 {}", date.month, date.day, wd),
        (LocaleCode::Ja, false) => format!("{}年{}月{}日", date.year, date.month, date.day),
    }
}

pub fn format_event_count(locale: LocaleCode, count: usize) -> String {
    match locale {
        LocaleCode::En => match count {
            0 => "No events".to_string(),
            1 => "1 event".to_string(),
            n => format!("{n} events"),
        },
        LocaleCode::Ru => {
            if count == 0 {
                return "Нет событий".to_string();
            }
            let rem100 = count % 100;
            let rem10 = count % 10;
            let word = if (11..=14).contains(&rem100) {
                "событий"
            } else if rem10 == 1 {
                "событие"
            } else if (2..=4).contains(&rem10) {
                "события"
            } else {
                "событий"
            };
            format!("{count} {word}")
        }
        LocaleCode::Es => match count {
            0 => "Sin eventos".to_string(),
            1 => "1 evento".to_string(),
            n => format!("{n} eventos"),
        },
        LocaleCode::Ja => match count {
            0 => "予定なし".to_string(),
            n => format!("予定{n}件"),
        },
    }
}

pub fn format_calendar_count(locale: LocaleCode, count: usize) -> String {
    match locale {
        LocaleCode::En => match count {
            1 => "1 calendar".to_string(),
            n => format!("{n} calendars"),
        },
        LocaleCode::Ru => {
            let rem100 = count % 100;
            let rem10 = count % 10;
            let word = if (11..=14).contains(&rem100) {
                "календарей"
            } else if rem10 == 1 {
                "календарь"
            } else if (2..=4).contains(&rem10) {
                "календаря"
            } else {
                "календарей"
            };
            format!("{count} {word}")
        }
        LocaleCode::Es => match count {
            1 => "1 calendario".to_string(),
            n => format!("{n} calendarios"),
        },
        LocaleCode::Ja => format!("カレンダー{count}件"),
    }
}

pub fn format_active_calendars_heading(locale: LocaleCode, count: usize) -> String {
    match locale {
        LocaleCode::En => format!("ACTIVE CALENDARS ({count})"),
        LocaleCode::Ru => format!("АКТИВНЫЕ КАЛЕНДАРИ ({count})"),
        LocaleCode::Es => format!("CALENDARIOS ACTIVOS ({count})"),
        LocaleCode::Ja => format!("有効なカレンダー ({count})"),
    }
}

pub fn format_sync_status(locale: LocaleCode, count: usize) -> String {
    let cal_count = format_calendar_count(locale, count);
    match locale {
        LocaleCode::En => format!("Synced · {cal_count} active"),
        LocaleCode::Ru => format!("Синхронизировано · {cal_count}"),
        LocaleCode::Es => format!("Sincronizado · {cal_count}"),
        LocaleCode::Ja => format!("同期済み · {cal_count}"),
    }
}

pub fn format_update_status(
    locale: LocaleCode,
    state: &crate::updater::UpdateCheckState,
) -> (String, bool) {
    use crate::updater::UpdateCheckState;
    let strings = catalog(locale);
    match state {
        UpdateCheckState::Idle => (strings.updates_idle_desc.to_string(), false),
        UpdateCheckState::Checking => (strings.updates_checking_desc.to_string(), false),
        UpdateCheckState::UpToDate { current_version } => match locale {
            LocaleCode::En => (format!("Up to date (v{current_version})"), false),
            LocaleCode::Ru => (
                format!("Установлена последняя версия (v{current_version})"),
                false,
            ),
            LocaleCode::Es => (format!("Actualizado (v{current_version})"), false),
            LocaleCode::Ja => (format!("最新バージョンです (v{current_version})"), false),
        },
        UpdateCheckState::UpdateAvailable { latest_version, .. } => match locale {
            LocaleCode::En => (format!("Update available: v{latest_version}"), true),
            LocaleCode::Ru => (format!("Доступно обновление: v{latest_version}"), true),
            LocaleCode::Es => (format!("Actualización disponible: v{latest_version}"), true),
            LocaleCode::Ja => (format!("アップデートがあります: v{latest_version}"), true),
        },
        UpdateCheckState::NoStableRelease => match locale {
            LocaleCode::En => (
                "No stable release published on GitHub yet.".to_string(),
                false,
            ),
            LocaleCode::Ru => ("Стабильных релизов на GitHub пока нет.".to_string(), false),
            LocaleCode::Es => (
                "Aún no hay versiones estables en GitHub.".to_string(),
                false,
            ),
            LocaleCode::Ja => (
                "GitHub にまだ安定版リリースがありません。".to_string(),
                false,
            ),
        },
        UpdateCheckState::Offline => match locale {
            LocaleCode::En => ("Offline: could not reach GitHub.".to_string(), false),
            LocaleCode::Ru => (
                "Нет сети: не удалось подключиться к GitHub.".to_string(),
                false,
            ),
            LocaleCode::Es => (
                "Sin conexión: no se pudo acceder a GitHub.".to_string(),
                false,
            ),
            LocaleCode::Ja => (
                "オフライン: GitHub にアクセスできません。".to_string(),
                false,
            ),
        },
        UpdateCheckState::Timeout => match locale {
            LocaleCode::En => ("Update check timed out.".to_string(), false),
            LocaleCode::Ru => ("Превышено время ожидания ответа.".to_string(), false),
            LocaleCode::Es => (
                "Tiempo de espera de actualización agotado.".to_string(),
                false,
            ),
            LocaleCode::Ja => (
                "アップデートの確認がタイムアウトしました。".to_string(),
                false,
            ),
        },
        UpdateCheckState::RateLimited => match locale {
            LocaleCode::En => (
                "GitHub API rate limit reached. Try again later.".to_string(),
                false,
            ),
            LocaleCode::Ru => (
                "Лимит запросов к GitHub API исчерпан. Повторите позже.".to_string(),
                false,
            ),
            LocaleCode::Es => (
                "Límite de GitHub API alcanzado. Inténtelo más tarde.".to_string(),
                false,
            ),
            LocaleCode::Ja => (
                "GitHub API のレート制限に達しました。後でお試しください。".to_string(),
                false,
            ),
        },
        UpdateCheckState::InvalidResponse(msg) => (msg.clone(), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pluralization_and_date_formatting_all_four_languages() {
        let oct29 = LocalDate::new(2026, 10, 29).unwrap();
        assert_eq!(
            format_date_line(LocaleCode::En, oct29, true),
            "Thursday, 29 October"
        );
        assert_eq!(
            format_date_line(LocaleCode::Ru, oct29, true),
            "Четверг, 29 октября"
        );
        assert_eq!(
            format_date_line(LocaleCode::Es, oct29, true),
            "Jueves, 29 de octubre"
        );
        assert_eq!(
            format_date_line(LocaleCode::Ja, oct29, true),
            "10月29日 木曜日"
        );

        assert_eq!(format_event_count(LocaleCode::Ru, 1), "1 событие");
        assert_eq!(format_event_count(LocaleCode::Ru, 2), "2 события");
        assert_eq!(format_event_count(LocaleCode::Ru, 5), "5 событий");
        assert_eq!(format_event_count(LocaleCode::Ru, 21), "21 событие");
        assert_eq!(format_event_count(LocaleCode::Ja, 2), "予定2件");

        assert_eq!(
            format_active_calendars_heading(LocaleCode::Ru, 3),
            "АКТИВНЫЕ КАЛЕНДАРИ (3)"
        );
        assert_eq!(
            format_sync_status(LocaleCode::Es, 2),
            "Sincronizado · 2 calendarios"
        );
        let up_to_date = crate::updater::UpdateCheckState::UpToDate {
            current_version: "0.1.0".into(),
        };
        assert_eq!(
            format_update_status(LocaleCode::Ru, &up_to_date).0,
            "Установлена последняя версия (v0.1.0)"
        );
    }
}
