use std::{future::poll_fn, pin::pin, task::Poll};

pub(crate) async fn unless_cancelled<F: Future>(future: F, cancelled: impl Fn() -> bool) -> Option<F::Output> {
    let mut future = pin!(future);
    poll_fn(|context| {
        if cancelled() {
            return Poll::Ready(None);
        }
        future.as_mut().poll(context).map(Some)
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        future::poll_fn,
        task::Poll,
    };

    use super::unless_cancelled;

    #[test]
    fn a_finished_future_gives_its_output() {
        assert_eq!(futures::executor::block_on(unless_cancelled(async { 7 }, || false)), Some(7));
    }

    #[test]
    fn cancelling_between_polls_drops_the_future() {
        let polls = Cell::new(0);
        let pending_twice = poll_fn(|context| {
            polls.set(polls.get() + 1);
            context.waker().wake_by_ref();
            if polls.get() > 5 { Poll::Ready(()) } else { Poll::Pending }
        });
        let output = futures::executor::block_on(unless_cancelled(pending_twice, || polls.get() >= 2));
        assert_eq!((output, polls.get()), (None, 2));
    }
}
