use gpui_kit::{AppContext as _, Context, SharedString, Task};

#[derive(Clone, Debug)]
pub(crate) struct Failure(SharedString);

impl Failure {
    pub(crate) fn describe(error: &anyhow::Error) -> Self {
        Self(format!("{error:#}").into())
    }

    pub(crate) fn git_reason(error: &anyhow::Error) -> Self {
        let message = format!("{error:#}");
        let lines = || {
            message
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
        };
        let reason = lines()
            .rfind(|line| line.starts_with("fatal:") || line.starts_with("error:"))
            .or_else(|| lines().next_back());
        Self(reason.unwrap_or("unknown error").to_owned().into())
    }

    pub(crate) fn message(&self) -> SharedString {
        self.0.clone()
    }
}

#[derive(Default)]
pub(crate) enum Load<T> {
    #[default]
    Idle,
    Loading {
        _task: Task<()>,
    },
    Refreshing {
        value: T,
        _task: Task<()>,
    },
    Ready(T),
    Failed(Failure),
}

impl<T> Load<T> {
    pub(crate) fn value(&self) -> Option<&T> {
        match self {
            Load::Refreshing { value, .. } | Load::Ready(value) => Some(value),
            Load::Idle | Load::Loading { .. } | Load::Failed(_) => None,
        }
    }

    pub(crate) fn is_loading(&self) -> bool {
        matches!(self, Load::Loading { .. } | Load::Refreshing { .. })
    }

    pub(crate) fn failure(&self) -> Option<&Failure> {
        match self {
            Load::Failed(failure) => Some(failure),
            _ => None,
        }
    }

    pub(crate) fn load(&mut self, task: Task<()>) {
        *self = Load::Loading { _task: task };
    }

    pub(crate) fn reload(&mut self, task: Task<()>) {
        *self = match std::mem::take(self) {
            Load::Ready(value) | Load::Refreshing { value, .. } => {
                Load::Refreshing { value, _task: task }
            }
            Load::Idle | Load::Loading { .. } | Load::Failed(_) => Load::Loading { _task: task },
        };
    }

    pub(crate) fn finish(&mut self, result: anyhow::Result<T>) {
        *self = match result {
            Ok(value) => Load::Ready(value),
            Err(error) => Load::Failed(Failure::describe(&error)),
        };
    }

    pub(crate) fn clear(&mut self) {
        *self = Load::Idle;
    }
}

#[derive(Default)]
pub(crate) struct Job(Option<Task<()>>);

impl Job {
    pub(crate) fn is_running(&self) -> bool {
        self.0.is_some()
    }

    pub(crate) fn run(&mut self, task: Task<()>) {
        self.0 = Some(task);
    }

    pub(crate) fn finish(&mut self) {
        self.0 = None;
    }

    pub(crate) fn cancel(&mut self) {
        self.0 = None;
    }
}

pub(crate) fn run_in_background<V: 'static, R: Send + 'static>(
    cx: &mut Context<V>,
    work: impl FnOnce() -> R + Send + 'static,
    then: impl FnOnce(&mut V, R, &mut Context<V>) + 'static,
) -> Task<()> {
    cx.spawn(async move |this, cx| {
        let result = cx.background_spawn(async move { work() }).await;
        let _ = this.update(cx, |this, cx| then(this, result, cx));
    })
}

#[cfg(test)]
mod tests {
    use gpui_kit::Task;

    use super::{Failure, Load};

    #[test]
    fn a_reload_keeps_showing_the_last_value() {
        let mut load = Load::Ready(1);
        load.reload(Task::ready(()));
        assert_eq!(load.value(), Some(&1));
        assert!(load.is_loading());
        load.finish(Ok(2));
        assert_eq!(load.value(), Some(&2));
        assert!(!load.is_loading());
    }

    #[test]
    fn a_load_drops_the_last_value() {
        let mut load = Load::Ready(1);
        load.load(Task::ready(()));
        assert_eq!(load.value(), None);
        assert!(load.is_loading());
    }

    #[test]
    fn a_failure_replaces_the_value() {
        let mut load = Load::Ready(1);
        load.reload(Task::ready(()));
        load.finish(Err(anyhow::anyhow!("gone")));
        assert_eq!(load.value(), None);
        assert_eq!(
            load.failure().map(Failure::message).as_deref(),
            Some("gone")
        );
    }

    #[test]
    fn git_reasons_keep_only_the_last_line() {
        let error = anyhow::anyhow!("remote: Denied\nfatal: Authentication failed\n\n");
        assert_eq!(
            Failure::git_reason(&error).message(),
            "fatal: Authentication failed"
        );
        let hook = anyhow::anyhow!("Checking style\n  src/main.rs: tabs found\n");
        assert_eq!(
            Failure::git_reason(&hook).message(),
            "src/main.rs: tabs found"
        );
        let empty = anyhow::anyhow!("  \n");
        assert_eq!(Failure::git_reason(&empty).message(), "unknown error");
    }

    #[test]
    fn git_reasons_prefer_the_fatal_line_to_the_advice_after_it() {
        let error = anyhow::anyhow!(
            "fatal: Unable to create '.git/index.lock': File exists.\n\nAnother git process seems to be running.\nremove the file manually to continue.\n"
        );
        assert_eq!(
            Failure::git_reason(&error).message(),
            "fatal: Unable to create '.git/index.lock': File exists."
        );
    }
}
