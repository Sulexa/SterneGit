use std::time::SystemTime;

use futures::future::BoxFuture;
use url::Url;

use super::request::{ApiRequest, ApiResponse, NetError};
use crate::accounts::AccountId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ConnectorId(pub(crate) String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AccountView {
    pub(crate) id: AccountId,
    pub(crate) server: Url,
    pub(crate) endpoints: Vec<(String, Url)>,
    pub(crate) login: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone)]
pub(crate) struct Cancel(smol::channel::Receiver<()>);

pub(crate) struct CancelOnDrop {
    _sender: smol::channel::Sender<()>,
}

pub(crate) fn cancel_pair() -> (CancelOnDrop, Cancel) {
    let (sender, receiver) = smol::channel::bounded(1);
    (CancelOnDrop { _sender: sender }, Cancel(receiver))
}

impl Cancel {
    pub(crate) async fn cancelled(&self) {
        while self.0.recv().await.is_ok() {}
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.0.is_closed()
    }
}

pub(crate) trait HostServices: Send + Sync + 'static {
    fn account(&self, account: &AccountId) -> Option<AccountView>;
    fn request(
        &self,
        account: &AccountId,
        request: ApiRequest,
        cancel: Cancel,
    ) -> BoxFuture<'static, Result<ApiResponse, NetError>>;
    fn pause(&self, account: &AccountId, until: SystemTime);
    fn log(&self, connector: &ConnectorId, level: Level, message: &str);
    fn now(&self) -> SystemTime;
}
