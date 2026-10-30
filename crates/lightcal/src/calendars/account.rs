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
