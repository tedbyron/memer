//! Result extensions for tracing.

use tracing::error;

pub trait TraceErr<T> {
    fn or_trace(self);
    fn trace_err(self) -> anyhow::Result<T>;
}

impl<T, E> TraceErr<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn or_trace(self) {
        if let Err(e) = self {
            let e = anyhow::Error::from(e);
            error!("{e}");
        }
    }

    fn trace_err(self) -> anyhow::Result<T> {
        let res = self.map_err(Into::into);

        if let Err(ref e) = res {
            error!("{e}");
        }

        res
    }
}
