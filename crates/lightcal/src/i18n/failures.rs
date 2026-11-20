//! Presentation of provider diagnostics. Raw diagnostics stay locale independent;
//! interface messages are selected only when a view is synchronized.
use crate::preferences::LocaleCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarFailure {
    OutlookNotConfigured,
    GoogleNotConfigured,
    GoogleSecretMissing,
    CalendarName,
    Duplicate,
    Https,
    SignInTimeout,
    Cancelled,
    Reconnect,
    Unverified,
    Callback,
    CalendarTooLarge,
    CalendarLimit,
    CalendarInvalid,
    Network,
    ResponseLimit,
    Authentication,
    InvalidResponse,
    Credentials,
    Unavailable,
    Permission,
    AccountLimit,
}

#[derive(Clone, Default)]
pub struct CalendarError {
    diagnostic: String,
    kind: Option<CalendarFailure>,
}

impl CalendarError {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn is_empty(&self) -> bool {
        self.kind.is_none() && self.diagnostic.is_empty()
    }
    pub fn render(&self, locale: LocaleCode) -> String {
        if let Some(kind) = self.kind {
            return calendar_failure_text(locale, kind).into();
        }
        if self.is_empty() {
            return String::new();
        }
        self.diagnostic
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| render_line(locale, line))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
impl From<String> for CalendarError {
    fn from(diagnostic: String) -> Self {
        Self {
            diagnostic,
            kind: None,
        }
    }
}
impl From<&str> for CalendarError {
    fn from(diagnostic: &str) -> Self {
        diagnostic.to_string().into()
    }
}
impl From<CalendarFailure> for CalendarError {
    fn from(kind: CalendarFailure) -> Self {
        Self {
            kind: Some(kind),
            diagnostic: String::new(),
        }
    }
}

fn locale_index(locale: LocaleCode) -> usize {
    match locale {
        LocaleCode::En => 0,
        LocaleCode::Ru => 1,
        LocaleCode::Es => 2,
        LocaleCode::Ja => 3,
        LocaleCode::De => 4,
        LocaleCode::Fr => 5,
        LocaleCode::PtBr => 6,
        LocaleCode::Uk => 7,
        LocaleCode::ZhHans => 8,
        LocaleCode::Ko => 9,
    }
}

pub fn mac_calendars_title(locale: LocaleCode) -> &'static str {
    [
        "macOS calendars",
        "Календари macOS",
        "Calendarios de macOS",
        "macOS のカレンダー",
        "macOS-Kalender",
        "Calendriers macOS",
        "Calendários do macOS",
        "Календарі macOS",
        "macOS 日历",
        "macOS 캘린더",
    ][locale_index(locale)]
}

pub fn startup_failure_text(locale: LocaleCode, macos: bool) -> &'static str {
    if !macos {
        return super::startup_copy(locale).error;
    }
    [
        "Couldn’t change Open at login. Check Lightcal in System Settings → General → Login Items.",
        "Не удалось изменить запуск при входе. Проверьте Lightcal в Системных настройках → Основные → Объекты входа.",
        "No se pudo cambiar Abrir al iniciar sesión. Revisa Lightcal en Ajustes del Sistema → General → Ítems de inicio.",
        "ログイン時に開く設定を変更できませんでした。システム設定 → 一般 → ログイン項目で Lightcal を確認してください。",
        "Das Öffnen bei der Anmeldung konnte nicht geändert werden. Prüfen Sie Lightcal unter Systemeinstellungen → Allgemein → Anmeldeobjekte.",
        "Impossible de modifier l’ouverture à la connexion. Vérifiez Lightcal dans Réglages Système → Général → Ouverture.",
        "Não foi possível alterar a abertura ao iniciar sessão. Confira o Lightcal em Ajustes do Sistema → Geral → Itens de Início.",
        "Не вдалося змінити запуск при вході. Перевірте Lightcal у Системних параметрах → Загальні → Об’єкти входу.",
        "无法更改登录时打开设置。请在系统设置 → 通用 → 登录项中检查 Lightcal。",
        "로그인 시 열기 설정을 변경하지 못했습니다. 시스템 설정 → 일반 → 로그인 항목에서 Lightcal을 확인하세요.",
    ][locale_index(locale)]
}

pub fn calendar_failure_text(locale: LocaleCode, kind: CalendarFailure) -> &'static str {
    use CalendarFailure::*;
    let texts:[&str;10]=match kind {
        OutlookNotConfigured => [
            "Outlook sign-in isn’t available in this copy of Lightcal.",
            "Вход в Outlook недоступен в этой версии Lightcal.",
            "El inicio de sesión en Outlook no está disponible en esta versión de Lightcal.",
            "この Lightcal では Outlook にサインインできません。",
            "Die Outlook-Anmeldung ist in dieser Version von Lightcal nicht verfügbar.",
            "La connexion à Outlook n’est pas disponible dans cette version de Lightcal.",
            "O login no Outlook não está disponível nesta versão do Lightcal.",
            "Вхід в Outlook недоступний у цій версії Lightcal.",
            "此版本的 Lightcal 无法登录 Outlook。",
            "이 Lightcal 버전에서는 Outlook에 로그인할 수 없습니다.",
        ],
        GoogleNotConfigured | GoogleSecretMissing => [
            "Google sign-in isn’t available in this copy of Lightcal.",
            "Вход в Google недоступен в этой версии Lightcal.",
            "El inicio de sesión en Google no está disponible en esta versión de Lightcal.",
            "この Lightcal では Google にサインインできません。",
            "Die Google-Anmeldung ist in dieser Version von Lightcal nicht verfügbar.",
            "La connexion à Google n’est pas disponible dans cette version de Lightcal.",
            "O login no Google não está disponível nesta versão do Lightcal.",
            "Вхід у Google недоступний у цій версії Lightcal.",
            "此版本的 Lightcal 无法登录 Google。",
            "이 Lightcal 버전에서는 Google에 로그인할 수 없습니다.",
        ],
        CalendarName => [
            "Enter a calendar name with 1–100 characters.", "Введите название календаря длиной от 1 до 100 символов.",
            "Introduce un nombre de calendario de 1 a 100 caracteres.", "カレンダー名を1～100文字で入力してください。",
            "Geben Sie einen Kalendernamen mit 1–100 Zeichen ein.", "Saisissez un nom de calendrier de 1 à 100 caractères.",
            "Digite um nome de calendário com 1 a 100 caracteres.", "Введіть назву календаря від 1 до 100 символів.",
            "请输入 1–100 个字符的日历名称。", "캘린더 이름을 1~100자로 입력하세요.",
        ],
        Duplicate => [
            "This calendar subscription is already connected.", "Этот календарь уже подключён.",
            "Ya tienes una suscripción a este calendario.", "このカレンダーはすでに接続されています。",
            "Dieser Kalender ist bereits abonniert.", "Ce calendrier est déjà connecté.",
            "Este calendário já está conectado.", "Цей календар уже під’єднано.",
            "此日历订阅已连接。", "이미 연결된 캘린더 구독입니다.",
        ],
        Https => [
            "Enter a direct HTTPS calendar link without a username, password, or # fragment.",
            "Введите прямую HTTPS-ссылку на календарь без имени пользователя, пароля и части после #.",
            "Introduce un enlace HTTPS directo al calendario, sin usuario, contraseña ni fragmento #.",
            "ユーザー名、パスワード、# 以降を含まない、カレンダーへの直接の HTTPS リンクを入力してください。",
            "Geben Sie einen direkten HTTPS-Kalenderlink ohne Benutzername, Passwort oder #-Fragment ein.",
            "Saisissez un lien HTTPS direct vers le calendrier, sans identifiant, mot de passe ni fragment #.",
            "Digite um link HTTPS direto do calendário, sem usuário, senha ou fragmento #.",
            "Введіть пряме HTTPS-посилання на календар без імені користувача, пароля й частини після #.",
            "请输入直接指向日历的 HTTPS 链接，不含用户名、密码或 # 片段。",
            "사용자 이름, 비밀번호, # 부분이 없는 캘린더의 직접 HTTPS 링크를 입력하세요.",
        ],
        SignInTimeout => [
            "Sign-in timed out. Try connecting again.", "Время ожидания входа истекло. Подключитесь ещё раз.",
            "Se agotó el tiempo para iniciar sesión. Intenta conectarte de nuevo.", "サインインがタイムアウトしました。もう一度接続してください。",
            "Die Anmeldung hat zu lange gedauert. Verbinden Sie das Konto erneut.", "Le délai de connexion a expiré. Réessayez de connecter le compte.",
            "O tempo para fazer login acabou. Tente conectar novamente.", "Час очікування входу минув. Спробуйте під’єднатися ще раз.",
            "登录超时。请重新连接。", "로그인 시간이 초과되었습니다. 다시 연결하세요.",
        ],
        Cancelled => [
            "Sign-in was cancelled. Your accounts are unchanged.", "Вход отменён. Аккаунты не изменены.",
            "Se canceló el inicio de sesión. Tus cuentas no han cambiado.", "サインインをキャンセルしました。アカウントは変更されていません。",
            "Die Anmeldung wurde abgebrochen. Ihre Konten bleiben unverändert.", "La connexion a été annulée. Vos comptes sont inchangés.",
            "O login foi cancelado. Suas contas não foram alteradas.", "Вхід скасовано. Облікові записи не змінено.",
            "登录已取消。你的账户未更改。", "로그인이 취소되었습니다. 계정은 변경되지 않았습니다.",
        ],
        Reconnect => [
            "Your sign-in has expired. Connect the account again.", "Срок действия входа истёк. Подключите аккаунт заново.",
            "Tu sesión ha caducado. Vuelve a conectar la cuenta.", "サインインの有効期限が切れました。アカウントを接続し直してください。",
            "Ihre Anmeldung ist abgelaufen. Verbinden Sie das Konto erneut.", "Votre session a expiré. Reconnectez le compte.",
            "Sua sessão expirou. Conecte a conta novamente.", "Термін дії входу минув. Під’єднайте обліковий запис знову.",
            "登录已过期。请重新连接账户。", "로그인이 만료되었습니다. 계정을 다시 연결하세요.",
        ],
        Unverified => [
            "Sign-in couldn’t be verified. Try connecting again.", "Не удалось подтвердить вход. Подключитесь ещё раз.",
            "No se pudo verificar el inicio de sesión. Intenta conectarte de nuevo.", "サインインを確認できませんでした。もう一度接続してください。",
            "Die Anmeldung konnte nicht bestätigt werden. Verbinden Sie das Konto erneut.", "Impossible de vérifier la connexion. Réessayez de connecter le compte.",
            "Não foi possível verificar o login. Tente conectar novamente.", "Не вдалося підтвердити вхід. Спробуйте під’єднатися ще раз.",
            "无法验证登录。请重新连接。", "로그인을 확인하지 못했습니다. 다시 연결하세요.",
        ],
        Callback => [
            "Sign-in couldn’t start. Try again.", "Не удалось начать вход. Повторите попытку.",
            "No se pudo iniciar la sesión. Inténtalo de nuevo.", "サインインを開始できませんでした。もう一度お試しください。",
            "Die Anmeldung konnte nicht gestartet werden. Versuchen Sie es erneut.", "Impossible de démarrer la connexion. Réessayez.",
            "Não foi possível iniciar o login. Tente novamente.", "Не вдалося почати вхід. Спробуйте ще раз.",
            "无法开始登录。请重试。", "로그인을 시작하지 못했습니다. 다시 시도하세요.",
        ],
        CalendarTooLarge => [
            "This calendar exceeds the 8 MiB limit. Try a smaller export.", "Размер календаря превышает 8 МиБ. Попробуйте экспортировать меньше событий.",
            "El calendario supera el límite de 8 MiB. Prueba con una exportación más pequeña.", "カレンダーが8 MiBの上限を超えています。小さくして書き出してください。",
            "Der Kalender überschreitet die Grenze von 8 MiB. Versuchen Sie einen kleineren Export.", "Ce calendrier dépasse la limite de 8 Mio. Essayez un export plus petit.",
            "O calendário ultrapassa o limite de 8 MiB. Tente uma exportação menor.", "Розмір календаря перевищує 8 МіБ. Спробуйте експортувати менше подій.",
            "此日历超过 8 MiB 限制。请尝试导出更小的日历。", "캘린더가 8 MiB 제한을 초과했습니다. 더 작게 내보내세요.",
        ],
        CalendarLimit => [
            "This calendar has too many events. Try a smaller calendar.", "В календаре слишком много событий. Попробуйте календарь меньшего размера.",
            "Este calendario tiene demasiados eventos. Prueba con uno más pequeño.", "カレンダーの予定が多すぎます。小さなカレンダーをお試しください。",
            "Dieser Kalender enthält zu viele Termine. Versuchen Sie einen kleineren Kalender.", "Ce calendrier contient trop d’événements. Essayez un calendrier plus petit.",
            "Este calendário tem eventos demais. Tente um calendário menor.", "У календарі забагато подій. Спробуйте менший календар.",
            "此日历的日程过多。请尝试较小的日历。", "캘린더에 일정이 너무 많습니다. 더 작은 캘린더를 사용하세요.",
        ],
        CalendarInvalid => [
            "This calendar couldn’t be read. Check the ICS link or export it again.", "Не удалось прочитать календарь. Проверьте ICS-ссылку или экспортируйте его заново.",
            "No se pudo leer el calendario. Revisa el enlace ICS o vuelve a exportarlo.", "カレンダーを読み込めませんでした。ICS リンクを確認するか、書き出し直してください。",
            "Der Kalender konnte nicht gelesen werden. Prüfen Sie den ICS-Link oder exportieren Sie ihn erneut.", "Impossible de lire ce calendrier. Vérifiez le lien ICS ou exportez-le à nouveau.",
            "Não foi possível ler o calendário. Confira o link ICS ou exporte novamente.", "Не вдалося прочитати календар. Перевірте ICS-посилання або експортуйте його знову.",
            "无法读取此日历。请检查 ICS 链接或重新导出。", "캘린더를 읽지 못했습니다. ICS 링크를 확인하거나 다시 내보내세요.",
        ],
        Network => [
            "Couldn’t connect. Check your internet connection and try again.", "Не удалось подключиться. Проверьте интернет-соединение и повторите попытку.",
            "No se pudo conectar. Revisa tu conexión a internet e inténtalo de nuevo.", "接続できませんでした。インターネット接続を確認して、もう一度お試しください。",
            "Die Verbindung ist fehlgeschlagen. Prüfen Sie Ihre Internetverbindung und versuchen Sie es erneut.", "Connexion impossible. Vérifiez votre connexion internet et réessayez.",
            "Não foi possível conectar. Confira sua conexão com a internet e tente novamente.", "Не вдалося під’єднатися. Перевірте інтернет-з’єднання та спробуйте ще раз.",
            "无法连接。请检查网络连接后重试。", "연결하지 못했습니다. 인터넷 연결을 확인하고 다시 시도하세요.",
        ],
        ResponseLimit => [
            "The response was too large or incomplete. Try again.", "Ответ слишком большой или получен не полностью. Повторите попытку.",
            "La respuesta es demasiado grande o está incompleta. Inténtalo de nuevo.", "応答が大きすぎるか不完全です。もう一度お試しください。",
            "Die Antwort war zu groß oder unvollständig. Versuchen Sie es erneut.", "La réponse est trop volumineuse ou incomplète. Réessayez.",
            "A resposta é muito grande ou está incompleta. Tente novamente.", "Відповідь завелика або неповна. Спробуйте ще раз.",
            "响应过大或不完整。请重试。", "응답이 너무 크거나 불완전합니다. 다시 시도하세요.",
        ],
        Authentication => [
            "Sign-in failed. Try connecting the account again.", "Войти не удалось. Подключите аккаунт ещё раз.",
            "No se pudo iniciar sesión. Vuelve a conectar la cuenta.", "サインインできませんでした。アカウントを接続し直してください。",
            "Die Anmeldung ist fehlgeschlagen. Verbinden Sie das Konto erneut.", "La connexion a échoué. Réessayez de connecter le compte.",
            "Não foi possível fazer login. Conecte a conta novamente.", "Не вдалося ввійти. Під’єднайте обліковий запис ще раз.",
            "登录失败。请重新连接账户。", "로그인하지 못했습니다. 계정을 다시 연결하세요.",
        ],
        InvalidResponse => [
            "The calendar service sent an unreadable response. Try again.", "Сервис календаря прислал нечитаемый ответ. Повторите попытку.",
            "El servicio de calendario envió una respuesta que no se puede leer. Inténtalo de nuevo.", "カレンダーサービスの応答を読み取れませんでした。もう一度お試しください。",
            "Die Antwort des Kalenderdienstes konnte nicht gelesen werden. Versuchen Sie es erneut.", "La réponse du service de calendrier est illisible. Réessayez.",
            "Não foi possível ler a resposta do serviço de calendário. Tente novamente.", "Не вдалося прочитати відповідь сервісу календаря. Спробуйте ще раз.",
            "无法读取日历服务的响应。请重试。", "캘린더 서비스의 응답을 읽지 못했습니다. 다시 시도하세요.",
        ],
        Credentials => [
            "Lightcal couldn’t access saved connections. Try again.", "Lightcal не удалось получить доступ к сохранённым подключениям. Повторите попытку.",
            "Lightcal no pudo acceder a las conexiones guardadas. Inténtalo de nuevo.", "保存した接続情報にアクセスできませんでした。もう一度お試しください。",
            "Lightcal konnte nicht auf gespeicherte Verbindungen zugreifen. Versuchen Sie es erneut.", "Lightcal n’a pas pu accéder aux connexions enregistrées. Réessayez.",
            "O Lightcal não conseguiu acessar as conexões salvas. Tente novamente.", "Lightcal не вдалося отримати доступ до збережених підключень. Спробуйте ще раз.",
            "Lightcal 无法访问已保存的连接。请重试。", "Lightcal이 저장된 연결에 접근하지 못했습니다. 다시 시도하세요.",
        ],
        Unavailable => [
            "Couldn’t complete this calendar request. Try again.", "Не удалось выполнить запрос календаря. Повторите попытку.",
            "No se pudo completar la solicitud del calendario. Inténtalo de nuevo.", "カレンダーへの要求を完了できませんでした。もう一度お試しください。",
            "Die Kalenderanfrage konnte nicht abgeschlossen werden. Versuchen Sie es erneut.", "Impossible de terminer cette demande au calendrier. Réessayez.",
            "Não foi possível concluir a solicitação do calendário. Tente novamente.", "Не вдалося виконати запит календаря. Спробуйте ще раз.",
            "无法完成此日历请求。请重试。", "캘린더 요청을 완료하지 못했습니다. 다시 시도하세요.",
        ],
        Permission => [
            "Calendar access was denied. Check the calendar’s permissions.", "Доступ к календарю запрещён. Проверьте разрешения календаря.",
            "Se denegó el acceso al calendario. Revisa sus permisos.", "カレンダーへのアクセスが拒否されました。アクセス権を確認してください。",
            "Der Kalenderzugriff wurde verweigert. Prüfen Sie die Berechtigungen des Kalenders.", "L’accès au calendrier a été refusé. Vérifiez ses autorisations.",
            "O acesso ao calendário foi negado. Confira as permissões do calendário.", "Доступ до календаря заборонено. Перевірте дозволи календаря.",
            "日历访问被拒绝。请检查日历权限。", "캘린더 접근이 거부되었습니다. 캘린더 권한을 확인하세요.",
        ],
        AccountLimit => [
            "Remove an account before adding another.", "Удалите один аккаунт, прежде чем добавить новый.",
            "Elimina una cuenta antes de añadir otra.", "別のアカウントを追加する前に、既存のアカウントを削除してください。",
            "Entfernen Sie ein Konto, bevor Sie ein weiteres hinzufügen.", "Supprimez un compte avant d’en ajouter un autre.",
            "Remova uma conta antes de adicionar outra.", "Видаліть один обліковий запис, перш ніж додати новий.",
            "请先移除一个账户，再添加其他账户。", "다른 계정을 추가하기 전에 계정 하나를 삭제하세요.",
        ],
    };
    texts[locale_index(locale)]
}

fn http_status(raw: &str) -> Option<u16> {
    let (_, tail) = raw.split_once("HTTP ")?;
    let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
    let status = digits.parse::<u16>().ok()?;
    (100..600).contains(&status).then_some(status)
}

fn classify(raw: &str) -> CalendarFailure {
    use CalendarFailure::*;
    let text = raw.to_ascii_lowercase();
    if text.contains("enter a calendar name") {
        CalendarName
    } else if text.contains("already connected") {
        Duplicate
    } else if text.contains("outlook sign-in is not configured")
        || text.contains("microsoft desktop")
    {
        OutlookNotConfigured
    } else if text.contains("google desktop")
        || text.contains("google sign-in is not configured")
        || text.contains("google client secret")
    {
        GoogleNotConfigured
    } else if text.contains("sign-in timed out") {
        SignInTimeout
    } else if text.contains("cancelled") {
        Cancelled
    } else if crate::net::is_sign_in_required(raw)
        || http_status(raw) == Some(401)
        || text.contains("connect microsoft first")
        || text.contains("connect google first")
    {
        Reconnect
    } else if http_status(raw) == Some(403) {
        Permission
    } else if text.contains("invalid sign-in")
        || text.contains("authorization code")
        || text.contains("authorization-code")
        || text.contains("sign-in callback rejected")
    {
        Unverified
    } else if text.contains("sign-in socket")
        || text.contains("sign-in callback")
        || text.contains("sign-in port")
    {
        Callback
    } else if text.contains("invalid https")
        || text.contains("use an https")
        || text.contains("plain http")
    {
        Https
    } else if text.contains("8 mib") {
        CalendarTooLarge
    } else if text.contains("account") && text.contains("limit") {
        AccountLimit
    } else if text.contains("keychain")
        || text.contains("credentials")
        || text.contains("credential lock")
    {
        Credentials
    } else if text.contains("too large, incomplete") {
        ResponseLimit
    } else if text.contains("failed or timed out")
        || text.contains("network")
        || text.contains("offline")
        || text.contains("dns")
    {
        Network
    } else if text.contains("authentication request")
        || text.contains("token exchange")
        || text.contains("no refresh token")
        || text.contains("access token")
    {
        Authentication
    } else if text.contains("limit") || text.contains("too many") || text.contains("overflow") {
        CalendarLimit
    } else if text.contains("response") || text.contains("calendar without an id") {
        InvalidResponse
    } else if text.contains("icalendar")
        || text.contains("vcalendar")
        || text.contains("ics ")
        || text.contains("calendar date")
        || text.contains("calendar timezone")
        || text.contains("event duration")
    {
        CalendarInvalid
    } else {
        Unavailable
    }
}

fn render_line(locale: LocaleCode, raw: &str) -> String {
    // Counts remain structured in the canonical diagnostic until presentation,
    // including when a partial read accompanies another provider error.
    if let Some((error, count)) = raw.rsplit_once("; Google events not read: ") {
        if let Ok(count) = count.parse::<usize>() {
            return format!(
                "{}; {}",
                render_line(locale, error),
                crate::calendars::skipped_events_note(locale, "Google", count)
            );
        }
    }
    if let Some(count) = raw
        .strip_prefix("Google events not read: ")
        .and_then(|n| n.parse::<usize>().ok())
    {
        return crate::calendars::skipped_events_note(locale, "Google", count);
    }
    if let Some((name, note)) = raw.rsplit_once(": ") {
        if let Some((count, suffix)) = note.split_once(' ') {
            if matches!(
                suffix,
                "event could not be read" | "events could not be read"
            ) {
                if let Ok(count) = count.parse::<usize>() {
                    return crate::calendars::skipped_events_note(locale, name, count);
                }
            }
        }
    }
    let message = calendar_failure_text(locale, classify(raw));
    match http_status(raw) {
        Some(status) => format!("{message} (HTTP {status})"),
        None => message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_catalog_covers_every_language_without_raw_english_fallbacks() {
        use CalendarFailure::*;
        for locale in LocaleCode::ALL {
            let mac = startup_failure_text(locale, true);
            assert!(!mac.contains("Windows"));
            for kind in [
                OutlookNotConfigured,
                GoogleNotConfigured,
                GoogleSecretMissing,
                CalendarName,
                Duplicate,
                Https,
                SignInTimeout,
                Cancelled,
                Reconnect,
                Unverified,
                Callback,
                CalendarTooLarge,
                CalendarLimit,
                CalendarInvalid,
                Network,
                ResponseLimit,
                Authentication,
                InvalidResponse,
                Credentials,
                Unavailable,
                Permission,
                AccountLimit,
            ] {
                let message = calendar_failure_text(locale, kind);
                assert!(!message.is_empty());
                if locale != LocaleCode::En {
                    assert_ne!(
                        message,
                        calendar_failure_text(LocaleCode::En, kind),
                        "{locale:?} {kind:?}"
                    );
                }
            }
            if locale != LocaleCode::En {
                assert_ne!(mac, startup_failure_text(LocaleCode::En, true));
            }
        }
        assert_eq!(startup_failure_text(LocaleCode::Fr,true),"Impossible de modifier l’ouverture à la connexion. Vérifiez Lightcal dans Réglages Système → Général → Ouverture.");
    }

    #[test]
    fn known_diagnostics_select_actionable_failures_and_keep_http_status_only() {
        use CalendarFailure::*;
        for (raw, expected) in [
            ("Enter a calendar name (1–100 characters)", CalendarName),
            ("This subscription is already connected", Duplicate),
            (
                "Google sign-in timed out; try connecting again",
                SignInTimeout,
            ),
            (
                "Microsoft sign-in timed out; try connecting again",
                SignInTimeout,
            ),
            ("Calendar exceeds the 8 MiB limit", CalendarTooLarge),
            ("Calendar exceeds the 5000-component limit", CalendarLimit),
            ("Invalid iCalendar structure", CalendarInvalid),
            ("Invalid HTTPS address", Https),
            ("Invalid sign-in state", Unverified),
            ("Could not bind local sign-in callback", Callback),
            ("HTTPS request failed or timed out", Network),
            (
                "Response is too large, incomplete, or timed out",
                ResponseLimit,
            ),
            ("Invalid authentication response", InvalidResponse),
            (
                "macOS Keychain could not unlock Lightcal credentials (error 5)",
                Credentials,
            ),
        ] {
            assert_eq!(classify(raw), expected, "{raw}");
        }
        let error = CalendarError::from("Authentication request failed (HTTP 400): invalid_client");
        let french = error.render(LocaleCode::Fr);
        assert!(french.starts_with("La connexion a échoué."));
        assert!(french.contains("HTTP 400"));
        assert!(!french.contains("invalid_client"));
        assert_eq!(
            CalendarError::from("arbitrary internal diagnostic").render(LocaleCode::Ja),
            calendar_failure_text(LocaleCode::Ja, Unavailable)
        );
    }

    #[test]
    fn diagnostics_and_partial_counts_retranslate_without_another_provider_request() {
        let error = CalendarError::from(
            "Google calendar events failed: HTTP 503; Google events not read: 2",
        );
        let fr = error.render(LocaleCode::Fr);
        let ja = error.render(LocaleCode::Ja);
        assert!(fr.contains("HTTP 503") && fr.contains("2 événements"));
        assert!(ja.contains("HTTP 503") && ja.contains("2 件"));
        assert!(!fr.contains("events not read") && !ja.contains("events not read"));
        let skipped = CalendarError::from("Holidays: 3 events could not be read");
        assert_eq!(
            skipped.render(LocaleCode::Fr),
            "Holidays\u{a0}: 3 événements n’ont pas pu être lus"
        );
        let mut configured = CalendarError::from(CalendarFailure::OutlookNotConfigured);
        assert!(!configured.is_empty());
        assert_ne!(
            configured.render(LocaleCode::Fr),
            configured.render(LocaleCode::En)
        );
        configured.clear();
        assert!(configured.is_empty() && configured.render(LocaleCode::Fr).is_empty());
    }

    #[test]
    fn count_restart_and_korean_templates_describe_the_actual_action() {
        assert_eq!(
            super::super::connection_labels(LocaleCode::Ru)[8],
            "Установить и перезапустить"
        );
        for (count, expected) in [
            (0, "0 calendriers"),
            (1, "1 calendrier"),
            (2, "2 calendriers"),
        ] {
            assert_eq!(
                super::super::format_calendar_count(LocaleCode::Fr, count),
                expected
            );
        }
        for n in [0, 1, 2, 11, 100] {
            let text = super::super::local_settings_text(LocaleCode::Ko)
                .skipped
                .replace("{events}", &format!("일정 {n}개"));
            assert!(!text.contains("(를)") && !text.contains("을(를)"));
            assert!(text.contains(&format!("{n}개")));
        }
    }
}
