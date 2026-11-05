use std::{future::poll_fn, pin::pin, task::Poll};

use futures::{FutureExt as _, StreamExt as _, future::BoxFuture, stream::FuturesUnordered};

use super::{CallError, Stop};
use crate::net::{
    request::{ApiResponse, NetError},
    services::Cancel,
};

pub(super) const REQUESTS_AT_ONCE: usize = 16;
pub(super) const BODY_BYTES_AT_ONCE: usize = 32 * 1024 * 1024;

pub(super) type Journey = BoxFuture<'static, Result<ApiResponse, NetError>>;

struct Outstanding {
    id: u32,
    body_bytes: usize,
    landed: Option<ApiResponse>,
}

#[derive(Default)]
pub(super) struct Requests {
    sent: u32,
    flying: FuturesUnordered<BoxFuture<'static, (u32, Result<ApiResponse, NetError>)>>,
    outstanding: Vec<Outstanding>,
}

impl Requests {
    pub(super) fn sent(&self) -> usize {
        self.sent as usize
    }

    pub(super) fn make_room(&self, body_bytes: usize) -> Result<(), String> {
        if self.outstanding.len() >= REQUESTS_AT_ONCE {
            return Err(format!(
                "{REQUESTS_AT_ONCE} requests are already waiting to be read"
            ));
        }
        let flying_bytes: usize = self
            .outstanding
            .iter()
            .filter(|request| request.landed.is_none())
            .map(|request| request.body_bytes)
            .sum();
        if flying_bytes + body_bytes > BODY_BYTES_AT_ONCE {
            return Err("the request bodies on their way would pass 32 MB".to_owned());
        }
        Ok(())
    }

    pub(super) fn launch(&mut self, journey: Journey, body_bytes: usize) -> u32 {
        self.sent += 1;
        let id = self.sent;
        self.flying.push(journey.map(move |result| (id, result)).boxed());
        self.outstanding.push(Outstanding {
            id,
            body_bytes,
            landed: None,
        });
        id
    }

    pub(super) fn claim(&mut self, id: u32) -> Result<Option<ApiResponse>, String> {
        let at = self
            .outstanding
            .iter()
            .position(|request| request.id == id)
            .ok_or_else(|| format!("request {id} isn't waiting to be read"))?;
        Ok(match self.outstanding[at].landed {
            Some(_) => self.outstanding.remove(at).landed,
            None => None,
        })
    }

    pub(super) async fn arrival(&mut self, id: u32, cancel: &Cancel) -> Result<ApiResponse, CallError> {
        let mut cancelled = pin!(cancel.cancelled());
        poll_fn(|cx| {
            if cancelled.as_mut().poll(cx).is_ready() {
                return Poll::Ready(Err(CallError::Stopped(Stop::Cancelled)));
            }
            while let Poll::Ready(Some((landed, result))) = self.flying.poll_next_unpin(cx) {
                match result {
                    Ok(response) => self.land(landed, response),
                    Err(error) => return Poll::Ready(Err(CallError::Net(error))),
                }
            }
            match self.claim(id) {
                Ok(Some(response)) => Poll::Ready(Ok(response)),
                Ok(None) if !self.flying.is_empty() => Poll::Pending,
                Ok(None) | Err(_) => Poll::Ready(Err(CallError::Stopped(Stop::Trapped {
                    message: format!("request {id} was lost on its way"),
                    log: Vec::new(),
                }))),
            }
        })
        .await
    }

    fn land(&mut self, id: u32, response: ApiResponse) {
        if let Some(request) = self.outstanding.iter_mut().find(|request| request.id == id) {
            request.landed = Some(response);
        }
    }
}
