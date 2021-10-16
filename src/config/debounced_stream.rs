use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use anyhow::Result;
use futures_util::Stream;
use tokio::time::{Duration, Instant, Sleep};

use crate::config::ProxyConfig;

pub struct DebouncedStream<S> {
    stream: S,
    delay: Duration,
    config_updated: Option<(ProxyConfig, Pin<Box<Sleep>>)>,
}

impl<S> DebouncedStream<S> {
    pub fn new(stream: S, delay: Duration) -> DebouncedStream<S> {
        Self {
            stream,
            delay,
            config_updated: None,
        }
    }
}

impl<S> Stream for DebouncedStream<S>
where
    S: Stream<Item = Result<ProxyConfig>> + Send + Unpin,
{
    type Item = Result<ProxyConfig>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        loop {
            if let Some((_, delay)) = &mut this.config_updated {
                if let Poll::Ready(()) = Pin::new(delay).poll(cx) {
                    let (config, _) = this.config_updated.take().unwrap();
                    return Poll::Ready(Some(Ok(config)));
                }
            }

            match Pin::new(&mut this.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(new_config))) => match &mut this.config_updated {
                    Some((config, delay)) => {
                        delay.as_mut().reset(Instant::now() + this.delay);
                        *config = new_config;
                    }
                    None => {
                        this.config_updated =
                            Some((new_config, Box::pin(tokio::time::sleep(this.delay))));
                    }
                },
                // Poll::Ready(None) | Poll::Ready(Some(Err(_))) | Poll::Pending
                res => return res,
            }
        }
    }
}
