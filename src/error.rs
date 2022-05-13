//! Error utilities

use std::fmt;

use tracing::error;

pub trait TraceErr {
    fn or_trace(self);
    fn trace_err(self) -> Self;
}

impl<T, E> TraceErr for Result<T, E>
where
    E: fmt::Debug,
{
    fn or_trace(self) {
        if let Err(e) = self {
            error!("{:?}", e);
        }
    }

    fn trace_err(self) -> Self {
        if let Err(ref e) = self {
            error!("{:?}", e);
        }

        self
    }
}
