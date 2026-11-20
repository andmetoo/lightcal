# Translation review

Date: 2026-11-09. Status: editorial brief for the native review required by [ADR 0014](decisions/0014-ten-languages.md). No catalog string was changed by this review.

Lightcal already has a full interface in English, German, Spanish, French, Brazilian Portuguese, Russian, Ukrainian, Japanese, Simplified Chinese and Korean. German and Japanese are the closest to something a person would write. Russian and Ukrainian are fluent and then drop into Windows jargon. Spanish changes between formal and informal you. Korean, and parts of Chinese and Portuguese, still read as a line-by-line transfer from English. The English source has the same problem in a different form: several strings are written for the developer who built the screen, which is the voice Apple’s writing guidance tells an app to leave behind.

Correct the English first, then translate from that English. Patching today’s translations against today’s English will preserve the drift.

## Where the text lives

Every user-visible sentence is hand-written Rust. There is no gettext or string catalog outside the code.

| Source | What it holds |
| --- | --- |
| [`crates/lightcal/src/i18n.rs`](../crates/lightcal/src/i18n.rs) | The four catalogs `UiStrings`, `SettingsText`, `LocalEventText` and `LocalSettingsText`, plus dates, plurals, agenda, tray, menu bar, durations and shortcut conflicts |
| [`crates/lightcal/src/main.rs`](../crates/lightcal/src/main.rs) | A second copy of the account, sign-in, ICS and update sentences, the compact-date line, and the Google status lines |
| [`crates/lightcal/src/preferences.rs`](../crates/lightcal/src/preferences.rs) | Language names, and warnings shown when the preferences file cannot be read |
| [`crates/lightcal/ui/app.slint`](../crates/lightcal/ui/app.slint), [`calendar.slint`](../crates/lightcal/ui/calendar.slint), [`settings.slint`](../crates/lightcal/ui/settings.slint) | English defaults, including an older settings vocabulary, and a hardcoded section title `macOS` |
| [`crates/lightcal/src/net.rs`](../crates/lightcal/src/net.rs) | Provider failures. They reach the account cards in English in every language |

Language names in the menu are already the names people use for their own language (`English`, `Русский`, `日本語`, `简体中文`, `Português (Brasil)`). Leave them.

Proper nouns stay as written: Lightcal, Google, Outlook, GitHub, Microsoft, Windows, macOS, HTTPS, GPL-3.0-only, koona.app, Rust, Slint, and `Client ID` (that is the label on Microsoft’s own portal). `ICS` can stay on the address field, next to a plain description.

## The voice

Apply Apple’s [Writing](https://developer.apple.com/design/human-interface-guidelines/writing) guidance to every in-window string, and the macOS menu rules to the native menu only.

In the window:

- Sentence case. Capitalize the first word and proper nouns. `Retry Sync`, `Open Privacy Settings` and `SELECTED DATE` become `Try again`, `Open System Settings` and `Selected`.
- A button names the action: `Sign in`, `Add account`, `Delete`, `Try again`. `OK`, `Yes` and `Dismiss` are the weak forms.
- A label is a noun or a short phrase with no period: `Week numbers`, `All day`, `Not set`.
- An explanation or an error is a complete sentence and ends with a period. The same role uses the same punctuation in every language. Today some setting descriptions end with a period and the neighboring one does not.
- Say what happened and what to do next. `Something went wrong. Try again.` replaces `The operation failed unexpectedly. Please try again.`
- Use contractions in English (`you’re`, `couldn’t`, `don’t`).
- One product word for one thing. The window is Settings, including the subtitle that currently says Preferences. Apple retired Preferences.
- The plus and the external-link arrow already exist as icons. The strings `+ Add account` and `Open release page ↗` should be `Add account` and `View release`.
- An action that needs more input uses the ellipsis character `…` (U+2026), not three periods. `Set…`, `Settings…`, `Check for Updates…`.
- No `please`, no `we`, no `simply`, no `just`, no exclamation marks. `Your browser is open.` replaces `We opened your browser.`
- Keep technical identity on the About page, in plain words: `Built with Rust and Slint, without an embedded browser.` `zero webview bloat` is a developer’s aside.

On the macOS menu bar, follow the menu conventions instead: title case, a trailing ellipsis when the command opens a window, `Settings…`, `Check for Updates…`, `Quit Lightcal`. Other languages capitalize the way that language capitalizes a menu item (first word only in Russian, Spanish, French and Portuguese). `Открыть Календарь` and `Abrir Calendario` should be `Открыть календарь` and `Abrir calendario`.

Address the reader the way Apple does in that language, and do it in every sentence:

| Language | Form |
| --- | --- |
| German | Sie. This catalog is already consistent. |
| French | vous. Drop `Veuillez`. `Réessayez.` is enough. |
| Spanish | tú everywhere. The catalog currently mixes `Presione` / `Pegue` with `Haz clic` / `Inténtalo`. |
| Portuguese (Brazil) | você. `Conclua o início de sessão` replaces `Conclua o login`. |
| Russian, Ukrainian | вы. Impersonal lines such as `Нажмите сочетание клавиш…` already fit. |
| Japanese | です/ます in sentences. Buttons stay short (`保存`, `キャンセル`, `削除`). |
| Korean | 해요체 (`하세요`), which most of the catalog already uses. |
| Chinese | 你, which the catalog already uses. |

Platform chrome uses the platform’s word. One English sentence cannot serve both the menu bar and the notification area.

| Idea | macOS | Windows |
| --- | --- | --- |
| Where the icon lives | menu bar | notification area |
| Open the system pane | Open System Settings | Open Windows privacy settings |
| Sign-in launch | Open at login | Open at login |
| Quit | Quit Lightcal | Quit Lightcal |

`desc_startup` is still `Start Lightcal quietly in the system tray when you sign in` on macOS. Only the label is swapped. The description needs the same split: `Lightcal opens in the menu bar when you sign in.` and `Lightcal opens in the notification area when you sign in.`

The accounts section title `macOS` in `settings.slint` is a developer label. The two buttons under it already say what they do. Remove the title.

`Hide to tray` becomes `Hide` on both platforms. The icon remains; the word `tray` does not tell a Mac user anything.

`Color` and `Monochrome` describe the icon. `Mono`, Spanish `gris` and Korean `흑백` each describe something else (`gris` is gray, `흑백` is black and white). On the menu bar, monochrome means the icon takes the menu bar’s color.

## Glossary

Use these words in Lightcal. They match Calendar and System Settings closely enough that a translated screen does not feel like a different app. Where Windows and macOS name the Settings app differently, use the in-app word below for Lightcal’s own window, and the platform word only on the button that opens the system pane.

| Idea | en | de | fr | es | pt-BR | ru | uk | ja | zh-Hans | ko |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Settings | Settings | Einstellungen | Réglages | Ajustes | Ajustes | Настройки | Налаштування | 設定 | 设置 | 설정 |
| Account | Account | Konto | Compte | Cuenta | Conta | Учётная запись | Обліковий запис | アカウント | 账户 | 계정 |
| Sign in | Sign in | Anmelden | Se connecter | Iniciar sesión | Iniciar sessão | Войти | Увійти | サインイン | 登录 | 로그인 |
| Event | Event | Termin | Événement | Evento | Evento | Событие | Подія | 予定 | 日程 | 일정 |
| All day | All day | Ganztägig | Toute la journée | Todo el día | Dia inteiro | Весь день | Увесь день | 終日 | 全天 | 종일 |
| Today | Today | Heute | Aujourd’hui | Hoy | Hoje | Сегодня | Сьогодні | 今日 | 今天 | 오늘 |
| Read-only | Read-only | Schreibgeschützt | Lecture seule | Solo lectura | Somente leitura | Только чтение | Лише читання | 読み取り専用 | 只读 | 읽기 전용 |
| Offline | Offline | Offline | Hors connexion | Sin conexión | Sem conexão | Нет сети | Немає мережі | オフライン | 离线 | 오프라인 |
| Cancel | Cancel | Abbrechen | Annuler | Cancelar | Cancelar | Отмена | Скасувати | キャンセル | 取消 | 취소 |
| Delete | Delete | Löschen | Supprimer | Eliminar | Excluir | Удалить | Видалити | 削除 | 删除 | 삭제 |
| Close | Close | Schließen | Fermer | Cerrar | Fechar | Закрыть | Закрити | 閉じる | 关闭 | 닫기 |
| Menu bar | menu bar | Menüleiste | barre des menus | barra de menús | barra de menus | строка меню | рядок меню | メニューバー | 菜单栏 | 메뉴 막대 |
| Notification area | notification area | Infobereich | zone de notification | área de notificación | área de notificação | область уведомлений | область сповіщень | 通知領域 | 通知区域 | 알림 영역 |
| Koi pond | Koi pond | Koiteich | Bassin de koïs | Estanque de kois | Lago de kois | Пруд с кои | Ставок із кої | 錦鯉の池 | 锦鲤池 | 코이 연못 |

`Termin`, `予定`, `日程` and `일정` are the right localizations of `event`. Translating them back toward the English word (`事件`, `이벤트`, `appointment`) is the machine-translation move.

`Koi` stays a name. `Lago de carpas`, `잉어 연못`, `Пруд с карпами кои` and `鯉の池` describe a generic carp pond. Chinese `锦鲤池` and German `Koiteich` are already right.

Russian currently says `аккаунт` while Ukrainian says `обліковий запис`. Both catalogs should use the account word in the glossary. `аккаунт` is widely understood and still reads as a borrowed label on a settings screen.

Connect and sign-in are different actions. `Connect Outlook` is `Outlook verbinden`, `Connecter Outlook`, `Подключить Outlook`. `Sign in` is the browser step. Russian `Войти в Outlook` on the connect button collapses the two.

## English to rewrite first

Strings absent from this table can stay, apart from the punctuation rule above. Slint defaults that still say `General`, `Appearance`, `Calendars`, `Global shortcut`, `Windows Calendar Store` or `Alabaster Light` are an older screen. After startup they are overwritten from the catalog; until then, and in any path that forgets a property, they are the interface. Make the defaults the same sentences as the English catalog.

| Where | Current | Replacement |
| --- | --- | --- |
| `preferences_subtitle` | Preferences | Remove the subtitle, or name the real section. The window is already Settings. |
| `selected_date_overline` | SELECTED DATE | Selected |
| `format_active_calendars_heading` | ACTIVE CALENDARS (3) | Active calendars (3) |
| `label_shortcut` | Open shortcut | Keyboard shortcut |
| `desc_shortcut` | Toggle the calendar popover from anywhere on the desktop | Show or hide the calendar from anywhere. |
| `shortcut_record` | Record… | Set… |
| `label_gray_weekdays` | Shaded days | Shaded weekdays |
| `desc_gray_weekdays` | Tint selected weekday columns with a soft vertical band | Shade the weekday columns you choose. |
| `desc_show_week_numbers` | Display ISO week numbers along the left margin | Show week numbers on the left. |
| `desc_language` | Interface language for month names, weekdays, and controls | Language for dates, weekdays and settings. |
| `calendars_dot_note` | Only checked calendars contribute event dots (up to 3 dots per day). | Checked calendars show up to three dots on a day. |
| `read_only_note` | Read-only. Lightcal never changes your events. | Lightcal can read events. It can’t change them. |
| `signing_note` | We opened your browser. Lightcal asks only for read access to your calendars. | Your browser is open. Lightcal asks only to read your calendars. |
| `worker_failed_error` | The operation failed unexpectedly. Please try again. | Something went wrong. Try again. |
| `retry_sync_button` | Retry Sync | Try again |
| `open_privacy_settings_button` | Open Privacy Settings | Open System Settings, or Open Windows privacy settings |
| `add_calendar_button` | + Add account | Add account |
| `back_to_calendar` | ‹ Back to settings | Back |
| `palette_monochrome` | Mono | Monochrome |
| `dismiss_label` | Dismiss | Close |
| `about_tagline` | A small calendar for the tray | A small calendar for the menu bar, or for the notification area |
| `desc_license_source` | GPL-3.0-only · Built with Rust, Slint, and zero webview bloat | GPL-3.0-only. Built with Rust and Slint, without an embedded browser. |
| `updates_checking_desc` | Checking GitHub for the latest stable release… | Checking for updates… |
| `manual_update_notes` | This build cannot install updates by itself. Download the newer release from GitHub. | This copy of Lightcal can’t install updates. Download the new version from GitHub. |
| `shortcut_conflict_title` | Shortcut unavailable: Alt + Space is already registered by Raycast. | Use the dynamic sentence already in `shortcut_conflict_text`. The Raycast line is sample copy. |
| `hide_to_tray` | Hide to tray | Hide |
| `tray` menu `exit` | Exit | Quit Lightcal |
| `mac_labels` Short / Full | Short, Full | Abbreviated, Full name |
| `skipped_google_events` | Google events not read: 3 | Couldn’t read 3 Google events. |
| Google status | Google session expired — sign in again | Your Google sign-in expired. Sign in again. |
| Preferences warning | Preferences schema v2 is newer than supported v1; the original file is kept and safe defaults were loaded. | These settings were saved by a newer Lightcal. The file was kept, and Lightcal opened with its usual settings. |
| `delete_question` | Delete event? | Delete this event? |
| `err_title` | Add a title. | Add a title. This one is already right. |

`Check now`, `Today`, `All day`, `Not set`, `Cancel`, `Save`, `Up to date` and `You’re up to date` can stay. They are already the way Settings and Calendar speak.

Weekday and month names are correct in all ten languages, including Ukrainian and Russian genitive dates (`30 октября`, `30 жовтня`). Keep those formatters. Two date bugs are worth fixing with the copy:

- French uses `1 mars`. The first of the month is `1er mars`.
- `format_calendar_count` says `0 calendrier`. Zero takes the plural: `0 calendriers`.
- English dates are day-month (`Friday, 30 October`, `Fri 13 Nov`). That is natural outside the United States and surprising inside it. One `en` catalog has to choose; document the choice next to `format_date_line` so it is not “fixed” later by accident. Numeric dates that the system already formats should follow the system locale.

`format_compact_summary` uppercases the whole line (`MON, OCT 30 · 2 EVENTS`, and the same `.to_uppercase()` for German, French, Portuguese and Ukrainian). Case is part of the translation. German `ß`, French accents and the mixed-case event phrase all suffer, and the shout is the same all-caps problem as `SELECTED DATE`. Japanese, Chinese and Korean already skip it. Format the compact line the way `format_short_date` does, and let the layout set the weight.

## Same sentence, two wordings

These pairs describe one control and currently disagree. After the English rewrite, each pair becomes one string in `i18n.rs`. The match arms in `main.rs` (account chooser, sign-in, ICS, update description, Google status) are the second copy and the reason translations drift.

| Meaning | One current wording | The other |
| --- | --- | --- |
| Outlook action | `Connect Outlook` | Russian `Войти в Outlook` |
| Private link | `Private HTTPS ICS address` | Russian settings `Закрытая ссылка ICS`, connection label `Секретная HTTPS-ссылка ICS` (`секретная` means secret) |
| Install | `Install and restart` | Russian `Установить`, with the restart omitted |
| Windows calendars | `Windows calendars (legacy, optional)` | Russian drops `optional`; Spanish drops `legacy` |
| Language description | English names what the setting changes | Russian, Spanish and Japanese promise that it updates instantly |
| Clear and Reset | `Clear` and `Reset` | Russian uses `Сбросить` for both. Clear is `Очистить`; Reset stays `Сбросить`. |
| Add account | `Add account` | Russian settings button `Добавить` |
| Calendar access off | `Calendar access is turned off in System Privacy Settings` | Spanish `El acceso al calendario no está disponible`; Japanese `カレンダーへのアクセスが利用できません` |
| Permission body | `Lightcal needs permission to read calendar events. The month view still works without them.` | Russian talks about local events and drops the second sentence |
| About | `About` | German navigation `Info` |
| Refresh | `Refresh` | Spanish `Actualizar`, which is also the word for a software update. Use `Actualizar` for events and `Buscar actualizaciones` for the version check, and keep those two buttons apart. |
| Build | `this build` | Left as `Build`, `ビルド`, `компиляция`, `сборка`, `컴파일`/`빌드` in the Google-unavailable and update lines. Say `this copy of Lightcal`. |

Provider errors stay English on every account card (`main.rs` documents this next to `looks_offline`). Those sentences need the same rewrite as `worker_failed_error`, then a translation. Matching them by English keywords can stay as an implementation detail; the card still needs a localized sentence.

## German

The catalog is the most consistent: `Termin`, `Konto`, `Einstellungen`, `Ganztägig`, `Schreibgeschützt`, Sie, and `KW` for the week column. Keep that vocabulary.

| Current | Replacement |
| --- | --- |
| Aufnehmen… | Festlegen… |
| Mono | Einfarbig |
| Info (the About nav item) | Über |
| Dieser Build | Diese Version von Lightcal |
| ganz ohne Webview | ohne eingebetteten Browser |
| Google-Sitzung abgelaufen – bitte erneut anmelden | Ihre Google-Anmeldung ist abgelaufen. Melden Sie sich erneut an. |
| Infobereich, when the icon is on the Mac menu bar | Menüleiste |

`Ausblenden` is a good macOS Hide. The Windows description still wants `Infobereich`.

## French

Typography is already careful: `’`, narrow no-break spaces before `:`, `Aujourd’hui`, `Toute la journée`. Keep that, and use the same narrow space in every new sentence.

| Current | Replacement |
| --- | --- |
| Enregistrer… (the shortcut button; this is also Save) | Définir… |
| Ignorer | Fermer |
| Mettre en évidence le mois | Mettre en évidence le mois en cours |
| Bassin de carpes koï | Bassin de koïs |
| sans webview superflu | sans navigateur intégré |
| Veuillez réessayer. | Réessayez. |
| 0 calendrier | 0 calendriers |
| 1 mars, when the day is the first | 1er mars |
| Lancer avec Windows, on the Mac | Ouvrir à la connexion |
| zone de notification, on the Mac | barre des menus |

`Paramètres` is the Windows word and `Réglages` is the macOS word. Lightcal’s own window should say `Réglages` so it matches System Settings. A button that opens Windows Settings can say `Paramètres Windows`.

## Spanish

The local-event strings already use tú (`Haz clic`, `Inténtalo`, `Elimina`). Bring the rest over to tú. `Ajustes` matches Apple; `Configuración` matches Windows. Use `Ajustes` for Lightcal and `Configuración de privacidad de Windows` only for the Windows button.

| Current | Replacement |
| --- | --- |
| Presione una nueva combinación o haga clic en Restablecer. | Pulsa una combinación nueva o haz clic en Restablecer. |
| Pegue una dirección HTTPS ICS privada. | Pega un enlace privado de calendario (HTTPS). Se guarda solo en este equipo. |
| Grabar… | Definir… |
| Descartar | Cerrar |
| Color o gris | En color o monocromo |
| El acceso al calendario no está disponible | El acceso al calendario está desactivado en Ajustes del Sistema |
| Esta compilación no instala actualizaciones sola. | Esta copia de Lightcal no puede instalar las actualizaciones. Descarga la versión nueva en GitHub. |
| Estanque koi | Estanque de kois |
| Lightcal necesita permiso de lectura para mostrar eventos locales. | Lightcal necesita permiso para leer los eventos del calendario. El mes se sigue viendo igual. |
| Reintentar sincronización | Reintentar |

`Hoy`, `Todo el día`, `Calendario` and `Cuenta` can stay.

## Portuguese (Brazil)

`você` is already the right address, and `Dia inteiro`, `Hoje`, `Conta` and `Excluir` fit Calendar. `Configurações` is the Windows app; Lightcal’s window should say `Ajustes`, with `Configurações do Windows` reserved for the system button.

| Current | Replacement |
| --- | --- |
| Conclua o login em {provider} | Conclua o início de sessão no {provider} |
| Gravar… | Definir… |
| Dispensar | Fechar |
| Mono | Monocromático |
| Lago de carpas | Lago de kois |
| A visão do mês funciona sem eles. | O mês continua visível. |
| sem o peso de um webview | sem um navegador embutido |
| entrada de inicialização do Windows | item de inicialização do Windows |
| Offline. Mostrando eventos salvos às {time}. | Sem conexão. Mostrando os eventos salvos às {time}. |

## Russian

The grammar is in good shape: genitive months, the one/few/many plural, `Сегодня`, `Весь день`, `Настройки`. The tone slips into forum Russian (`трей`, `аккаунт`, `релиз`, `сборка`) and a few lines no longer mean what the English means.

| Current | Replacement |
| --- | --- |
| Войти в Outlook | Подключить Outlook |
| Секретная HTTPS-ссылка ICS | Закрытая ссылка ICS (HTTPS) |
| Установить | Установить и перезапустить |
| Календари Windows (устаревший способ) | Календари Windows (прежний способ, по желанию) |
| Сбросить, on Clear | Очистить |
| Сбросить, on Reset | Сбросить |
| Пруд с карпами кои | Пруд с кои |
| аккаунт, throughout | учётная запись |
| Значок в трее | Значок в области уведомлений, or в строке меню on macOS |
| Свернуть в трей | Скрыть |
| Выход, in the tray menu | Завершить Lightcal |
| Открыть Календарь | Открыть календарь |
| Маленький календарь в трее | Небольшой календарь у часов |
| Открыть релиз | Открыть выпуск |
| Доступно обновление, as a badge, can stay. `релиз` in prose cannot. | |
| Lightcal требуются разрешения на чтение для отображения локальных событий. | Lightcal запрашивает доступ, чтобы читать события календаря. Месяц виден и без этого доступа. |
| Мы открыли браузер. | Браузер открыт. Lightcal запрашивает только чтение календарей. |
| Вход через Google в этой сборке не настроен. | Вход через Google в этой копии Lightcal не настроен. |
| Сеанс Google истёк — войдите повторно | Срок входа в Google истёк. Войдите снова. |
| Не прочитано событий Google: {count} | Не удалось прочитать события Google: {count}. |
| Моно | Монохром |
| Автоматически устанавливать подписанные обновления | Автоматически устанавливать проверенные обновления |

`Задать…`, `Отмена`, `Сохранить`, `Удалить событие?` and the genitive dates can stay. `Подробнее` is the right face for `Details`.

## Ukrainian

Written as its own language, not as Russian with substituted letters: `застосунок`, `подія`, `обліковий запис`, `Налаштування`, genitive dates. Keep that. Align the verbs with Russian where the product action is the same, so the two catalogs do not invent different features.

| Current | Replacement |
| --- | --- |
| Ставок із коропами кої | Ставок із кої |
| Значок у треї / Сховати в трей | Значок в області сповіщень / Сховати. On the Mac: рядок меню. |
| Інсталювати й перезапустити | Встановити й перезапустити |
| Моно | Монохром |
| Вийти, where the menu means Quit | Завершити Lightcal |
| Підключити календарі… next to Під’єднати Outlook | Pick `під’єднати` for both. |
| Відкрити Календар | Відкрити календар |
| Схема / schema wording, if it ever mirrors the English warning | Ці параметри збережено новішою версією Lightcal. Файл залишено, і Lightcal відкрився зі звичайними параметрами. |

`Сьогодні`, `Увесь день`, `Скасувати` and `Подія` can stay.

## Japanese

`予定`, `カレンダー`, `終日`, `今日`, `設定` and `アカウント` are already the Calendar words. Politeness is in the right place. A few compounds are new words built out of the English.

| Current | Replacement |
| --- | --- |
| 記録… | 設定… |
| 陰影曜日, in the subtitle | 網掛けする曜日 |
| 予定ドット | 予定の印 |
| 鯉の池 | 錦鯉の池 |
| トレイに隠す | メニューバーにしまう, or 通知領域にしまう on Windows |
| このビルドは更新を自分ではインストールできません | この Lightcal はアップデートを自分ではインストールできません |
| ウェブビュー不使用 | 内蔵ブラウザーはありません |
| 完全, for the menu-bar weekday length | フルネーム |
| Lightcalを終了 | Lightcal を終了 |
| 進行中, if the chip means the current time | 現在 |
| 読み取れなかった Google の予定: {count} | Google の予定を {count} 件読み取れませんでした |

`サインイン`, `読み取り専用`, `キャンセル` and `終日` can stay. `今月` is better than a literal `現在の月`.

## Simplified Chinese

Two vocabularies are mixed. The main catalog says `事件` and `帐户`; the local-event catalog says `日程` and `帐户`. Calendar uses `日程` and `账户`. `脱机` is the old system word for what people now call `离线`. `单击` is the Windows click verb; on the Mac the sentence can avoid the pointer verb (`或轻点“重置”`).

Use `日程`, `账户`, `设置`, `登录`, `全天`, `今天`, `只读`, `离线` everywhere, including the strings that currently say `事件`, `帐户` or `脱机`.

| Current | Replacement |
| --- | --- |
| 所选日期, as an all-caps style label | 所选日期 is fine in Chinese. Drop the English capitalization rather than inventing emphasis. |
| 录制… | 设置… |
| 首选项 | Remove it with the English Preferences subtitle. |
| 托盘图标 | 菜单栏图标, or 通知区域图标 on Windows |
| 隐藏到托盘 | 隐藏 |
| 此版本无法自行安装更新 | 此 Lightcal 无法自行安装更新。请从 GitHub 下载新版本。 |
| 不含臃肿的 webview | 不含内嵌浏览器 |
| 发行说明 | 更新说明 |
| 未能读取的 Google 事件 | 未能读取 {count} 个 Google 日程 |
| Google 会话已过期，请重新登录 | Google 登录已过期，请重新登录 |
| 退出Lightcal | 退出 Lightcal |
| 简短 / 完整, for the menu-bar weekday | 缩写 / 全称 |

`锦鲤池`, `全天`, `今天` and `取消` can stay. `本月` is the natural badge.

## Korean

The grammar is usually correct, and the politeness level fits. The wording is still the English sentence with Korean words substituted: `컨트롤`, `트레이`, `팝업`, `배지`, `음영 요일`, `일정 점`, `잉어 연못`. Rewrite those lines as Korean, using the glossary, rather than swapping one word.

| Current | Replacement |
| --- | --- |
| 월 이름, 요일 및 컨트롤에 사용할 인터페이스 언어 | 날짜, 요일, 설정에 쓸 언어 |
| 바탕 화면 어디에서나 캘린더 팝업을 열거나 닫습니다 | 어디서든 캘린더를 열거나 가릴 수 있습니다 |
| 기록… | 설정… |
| 음영 요일 | 요일 음영 |
| 선택한 요일 열에 은은한 세로 띠를 표시합니다 | 고른 요일을 은은하게 표시합니다 |
| 일정 점 | 일정 표시 |
| 잉어 연못 | 코이 연못 |
| 컬러 또는 흑백 트레이 배지 | 색상 또는 단색 아이콘 |
| 기본 설정 | Remove it with Preferences. |
| 트레이로 숨기기 | 가리기 |
| 끝내기 | Lightcal 종료 |
| 이 빌드는 업데이트를 직접 설치할 수 없습니다 | 이 Lightcal은 업데이트를 직접 설치할 수 없습니다. GitHub에서 새 버전을 받으세요. |
| 무거운 webview 없음 | 내장 브라우저 없음 |
| 짧게 / 전체 | 짧게 / 요일 전체 |
| 읽지 못한 Google 일정: {count} | Google 일정 {count}개를 읽지 못했습니다 |
| 파일에서 {events}을(를) 읽을 수 없어 건너뛰었습니다 | 읽지 못해 건너뛴 일정: {events} |

`{events}` is already a full count phrase, such as `일정 3개`. A particle after that phrase has to agree with whatever the phrase happens to end with, which is why the catalog writes `을(를)`. Put the phrase where it does not need a particle (`읽지 못해 건너뛴 일정: {events}`). The same rule applies in every language: a placeholder needs a sentence that still reads correctly after the number and the noun are inserted. Slavic plurals already do this. Korean and Japanese counters should too.

`오늘`, `종일`, `캘린더`, `계정`, `취소` and `삭제` can stay.

## Order of work

1. Move the account, sign-in, ICS, update and Google sentences from `main.rs` into `i18n.rs`, so each sentence exists once.
2. Rewrite the English column in this document, including platform-specific startup, hide and icon strings.
3. Point the Slint defaults at those English sentences, and remove the `macOS` section title.
4. Translate from the new English with the glossary. A line that is already natural and still means the same thing can stay.
5. Apply the grammar notes: French `1er` and `0 calendriers`, one address form per language, no uppercase pass over translated text, ellipsis as `…`, French narrow spaces, Chinese `日程` / `账户` / `离线`.
6. Native read of each language in the real window, at the real width. ADR 0014 already makes that a release task. This document is the brief, not the sign-off. A string that fits in English can clip in German, and a reader of Ukrainian or Russian is the person who will throw out `трей`.
7. Refresh the snapshots in ADR 0014 after the copy changes. Snapshot tests will fail on purpose; that is the review of the new text.

Strings that are not listed above are acceptable. They still get a native read. They do not need a rewrite unless step 2 changes their English.
