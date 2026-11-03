//! Account state is independent of calendar availability and UI widgets.
use crate::preferences::LocaleCode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MicrosoftAccountState {
    Checking,
    Disconnected,
    Connecting,
    Connected { label: Option<String> },
    NeedsSignIn,
    Unavailable { saved_credentials: bool },
}

impl MicrosoftAccountState {
    pub fn can_disconnect(&self) -> bool {
        matches!(
            self,
            Self::Connected { .. }
                | Self::NeedsSignIn
                | Self::Unavailable {
                    saved_credentials: true
                }
        )
    }
    pub fn can_connect(&self) -> bool {
        matches!(
            self,
            Self::Disconnected | Self::NeedsSignIn | Self::Unavailable { .. }
        )
    }
    pub fn status_text(&self, locale: LocaleCode) -> String {
        let states = match locale {
            LocaleCode::En => [
                "Checking Outlook connection…",
                "Outlook is not connected",
                "Waiting for sign-in in your browser…",
                "Outlook connected",
                "Outlook session expired — sign in again",
                "Could not verify Outlook connection",
            ],
            LocaleCode::Ru => [
                "Проверка подключения Outlook…",
                "Outlook не подключён",
                "Ожидание входа в браузере…",
                "Outlook подключён",
                "Сеанс Outlook истёк — войдите повторно",
                "Не удалось проверить подключение Outlook",
            ],
            LocaleCode::Es => [
                "Comprobando conexión con Outlook…",
                "Outlook no está conectado",
                "Esperando el inicio de sesión en el navegador…",
                "Outlook conectado",
                "La sesión de Outlook caducó; vuelve a iniciar sesión",
                "No se pudo comprobar la conexión con Outlook",
            ],
            LocaleCode::Ja => [
                "Outlook 接続を確認中…",
                "Outlook は未接続です",
                "ブラウザーでのサインインを待機中…",
                "Outlook に接続済み",
                "Outlook のセッションが切れました。再度サインインしてください",
                "Outlook 接続を確認できませんでした",
            ],
            LocaleCode::De => [
                "Outlook-Verbindung wird geprüft…",
                "Outlook ist nicht verbunden",
                "Warten auf Anmeldung im Browser…",
                "Outlook verbunden",
                "Outlook-Sitzung abgelaufen – bitte erneut anmelden",
                "Outlook-Verbindung konnte nicht geprüft werden",
            ],
            LocaleCode::Fr => [
                "Vérification de la connexion Outlook…",
                "Outlook n’est pas connecté",
                "En attente de la connexion dans le navigateur…",
                "Outlook connecté",
                "La session Outlook a expiré — reconnectez-vous",
                "Impossible de vérifier la connexion Outlook",
            ],
            LocaleCode::PtBr => [
                "Verificando a conexão com o Outlook…",
                "O Outlook não está conectado",
                "Aguardando o login no navegador…",
                "Outlook conectado",
                "A sessão do Outlook expirou — entre novamente",
                "Não foi possível verificar a conexão com o Outlook",
            ],
            LocaleCode::Uk => [
                "Перевірка під’єднання Outlook…",
                "Outlook не під’єднано",
                "Очікування входу в браузері…",
                "Outlook під’єднано",
                "Сеанс Outlook завершився — увійдіть знову",
                "Не вдалося перевірити під’єднання Outlook",
            ],
            LocaleCode::ZhHans => [
                "正在检查 Outlook 连接…",
                "Outlook 未连接",
                "正在等待浏览器中的登录…",
                "Outlook 已连接",
                "Outlook 会话已过期，请重新登录",
                "无法验证 Outlook 连接",
            ],
            LocaleCode::Ko => [
                "Outlook 연결 확인 중…",
                "Outlook이 연결되지 않았습니다",
                "브라우저에서 로그인을 기다리는 중…",
                "Outlook 연결됨",
                "Outlook 세션이 만료되었습니다. 다시 로그인하세요",
                "Outlook 연결을 확인할 수 없습니다",
            ],
        };
        match self {
            Self::Checking => states[0].into(),
            Self::Disconnected => states[1].into(),
            Self::Connecting => states[2].into(),
            Self::Connected { label: Some(label) } => format!("{} · {label}", states[3]),
            Self::Connected { label: None } => states[3].into(),
            Self::NeedsSignIn => states[4].into(),
            Self::Unavailable { .. } => states[5].into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actions_follow_verified_account_state() {
        for state in [
            MicrosoftAccountState::Checking,
            MicrosoftAccountState::Connecting,
        ] {
            assert!(!state.can_disconnect());
            assert!(!state.can_connect());
        }
        assert!(!MicrosoftAccountState::Disconnected.can_disconnect());
        assert!(MicrosoftAccountState::Disconnected.can_connect());
        let connected = MicrosoftAccountState::Connected { label: None };
        assert!(connected.can_disconnect());
        assert!(!connected.can_connect());
        assert!(MicrosoftAccountState::NeedsSignIn.can_disconnect());
        assert!(MicrosoftAccountState::NeedsSignIn.can_connect());
        assert!(!MicrosoftAccountState::Unavailable {
            saved_credentials: false
        }
        .can_disconnect());
    }
}
