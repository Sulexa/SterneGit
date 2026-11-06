use std::time::{Duration, SystemTime};

use sternegit_protocol::Failure;
use gpui_kit::{
    AnyElement, App, IntoElement, ParentElement as _, SharedString, Styled as _,
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, h_flex, v_flex},
    div, px,
};

use crate::{
    accounts::AccountId,
    app::connectors::Connectors,
    connect::{CallError, Stop},
    net::{request::NetError, services::ConnectorId},
    ui::Look as _,
};

const LOG_LINES_SHOWN: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fix {
    SignInAgain,
    SignInWithBrowser,
    Retry,
    CopyLog,
    Nothing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Error,
    Warning,
    Wait,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Trouble {
    pub(crate) title: SharedString,
    pub(crate) message: SharedString,
    pub(crate) log: Vec<String>,
    pub(crate) fix: Fix,
    pub(crate) tone: Tone,
}

impl Trouble {
    pub(crate) fn of(error: &CallError, account: &AccountId, cx: &App) -> Self {
        let connectors = Connectors::of(cx);
        let found = connectors.account(account);
        let login = found.as_ref().map_or_else(String::new, |account| account.login.clone());
        let service = found
            .as_ref()
            .and_then(|account| connectors.service_of(account))
            .map_or_else(|| "The server".to_owned(), |choice| choice.manifest.name.clone());
        let api_host = found
            .as_ref()
            .and_then(|account| account.endpoints.first())
            .and_then(|endpoint| endpoint.url.host_str().map(str::to_owned))
            .unwrap_or_else(|| "the server".to_owned());
        let log = found
            .as_ref()
            .map(|account| connectors.log_lines(&ConnectorId(account.connector.clone())))
            .unwrap_or_default();
        Self::described(error, &login, &service, &api_host, log)
    }

    pub(crate) fn described(error: &CallError, login: &str, service: &str, api_host: &str, log: Vec<String>) -> Self {
        let trouble = |title: String, message: String, fix: Fix, tone: Tone| Self {
            title: title.into(),
            message: message.into(),
            log: Vec::new(),
            fix,
            tone,
        };
        match error {
            CallError::Failed(Failure::SignedOut) | CallError::Net(NetError::NotSignedIn) => trouble(
                format!("{service} refused the token of {login}"),
                "It expired or was revoked. Sign in again to keep using this account.".to_owned(),
                Fix::SignInAgain,
                Tone::Error,
            ),
            CallError::Failed(Failure::Forbidden { needs, message }) => trouble(
                format!("{login}'s token can't do this"),
                match needs {
                    Some(needs) => format!("It lacks {needs}. Sign in again with a token that has it."),
                    None => message.clone(),
                },
                Fix::SignInWithBrowser,
                Tone::Warning,
            ),
            CallError::Failed(Failure::RateLimited { retry_after_seconds }) => trouble(
                format!("{service}'s rate limit for {login} is used up"),
                format!("SterneGit asks again {}.", in_minutes(Duration::from_secs(*retry_after_seconds))),
                Fix::Nothing,
                Tone::Wait,
            ),
            CallError::Net(NetError::RateLimitedUntil(until)) => trouble(
                format!("{service}'s rate limit for {login} is used up"),
                format!(
                    "SterneGit asks again {}.",
                    in_minutes(until.duration_since(SystemTime::now()).unwrap_or_default())
                ),
                Fix::Nothing,
                Tone::Wait,
            ),
            CallError::Failed(Failure::NotFound) => trouble(
                format!("Not found on {service}"),
                "The repository was deleted or renamed, or this account can't see it.".to_owned(),
                Fix::Nothing,
                Tone::Error,
            ),
            CallError::Failed(Failure::Unsupported) | CallError::Unavailable => trouble(
                "Not available".to_owned(),
                "This connector doesn't offer it.".to_owned(),
                Fix::Nothing,
                Tone::Wait,
            ),
            CallError::Failed(failure) => {
                trouble(format!("{service} refused"), failure.to_string(), Fix::Retry, Tone::Error)
            }
            CallError::Net(error) => {
                trouble(format!("Couldn't reach {api_host}"), error.to_string(), Fix::Retry, Tone::Error)
            }
            CallError::Stopped(stop) => {
                let mut stopped = trouble(
                    format!("The {service} connector stopped"),
                    stop.to_string(),
                    Fix::CopyLog,
                    Tone::Error,
                );
                stopped.log = match stop {
                    Stop::Trapped { log, .. } if !log.is_empty() => log.clone(),
                    _ => log,
                };
                let skip = stopped.log.len().saturating_sub(LOG_LINES_SHOWN);
                stopped.log.drain(..skip);
                stopped
            }
        }
    }

    pub(crate) fn render(&self, action: Option<AnyElement>, cx: &App) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let (icon, color) = match self.tone {
            Tone::Error => (IconName::CircleX, theme.red),
            Tone::Warning => (IconName::TriangleAlert, theme.yellow),
            Tone::Wait => (IconName::Clock, theme.muted_foreground),
        };
        let (muted, border, well) = (theme.muted_foreground, theme.border, theme.secondary);
        let mono = theme.mono_font_family.clone();
        let log = self.log.clone();
        h_flex()
            .items_start()
            .gap_3()
            .p_4()
            .border_1()
            .border_color(border)
            .rounded(px(8.))
            .child(div().mt_0p5().child(Icon::new(icon).small().text_color(color)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(div().sm_text().font_medium().child(self.title.clone()))
                    .child(div().xs_text().text_color(muted).child(self.message.clone()))
                    .children(action.map(|action| h_flex().mt_2().child(action)))
                    .children((!log.is_empty()).then(|| {
                        v_flex()
                            .mt_2()
                            .px_2p5()
                            .py_2()
                            .rounded(px(6.))
                            .bg(well)
                            .font_family(mono)
                            .xs_text()
                            .text_color(muted)
                            .children(log.into_iter().map(|line| div().ellipsis().child(line)))
                    })),
            )
    }
}

fn in_minutes(wait: Duration) -> String {
    match wait.as_secs().div_ceil(60) {
        0 | 1 => "in a minute".to_owned(),
        minutes => format!("in {minutes} minutes"),
    }
}

#[cfg(test)]
mod tests {
    use sternegit_protocol::Failure;

    use super::{Fix, Tone, Trouble};
    use crate::{
        connect::{CallError, Stop},
        net::request::NetError,
    };

    fn described(error: CallError) -> Trouble {
        Trouble::described(&error, "octocat", "GitHub", "api.github.com", vec!["debug one".into(), "warn two".into()])
    }

    #[test]
    fn a_refused_token_asks_to_sign_in_again() {
        let trouble = described(CallError::Failed(Failure::SignedOut));
        assert_eq!(trouble.title, "GitHub refused the token of octocat");
        assert_eq!(trouble.fix, Fix::SignInAgain);
        assert_eq!(described(CallError::Net(NetError::NotSignedIn)).fix, Fix::SignInAgain);
    }

    #[test]
    fn a_missing_scope_is_named() {
        let trouble = described(CallError::Failed(Failure::Forbidden {
            needs: Some("write:public_key".into()),
            message: "Not Found".into(),
        }));
        assert_eq!(trouble.message, "It lacks write:public_key. Sign in again with a token that has it.");
        assert_eq!((trouble.fix, trouble.tone), (Fix::SignInWithBrowser, Tone::Warning));
    }

    #[test]
    fn a_rate_limit_says_when_sternegit_asks_again() {
        let trouble = described(CallError::Failed(Failure::RateLimited { retry_after_seconds: 600 }));
        assert_eq!(trouble.message, "SterneGit asks again in 10 minutes.");
        assert_eq!(trouble.tone, Tone::Wait);
    }

    #[test]
    fn a_network_failure_names_the_server_and_offers_to_retry() {
        let trouble = described(CallError::Net(NetError::Unreachable("curl: (6) Could not resolve host".into())));
        assert_eq!(trouble.title, "Couldn't reach api.github.com");
        assert_eq!(trouble.fix, Fix::Retry);
    }

    #[test]
    fn a_stopped_connector_shows_its_last_log_lines() {
        let trouble = described(CallError::Stopped(Stop::OutOfFuel));
        assert_eq!(trouble.title, "The GitHub connector stopped");
        assert_eq!(trouble.log, ["debug one", "warn two"]);
        assert_eq!(trouble.fix, Fix::CopyLog);
    }
}
