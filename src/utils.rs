use std::future::Future;

pub fn wait_async<F: Future>(fut: F) -> F::Output {
    pollster::block_on(fut)
}
