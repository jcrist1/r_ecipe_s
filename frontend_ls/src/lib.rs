use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use futures::{Sink, SinkExt, StreamExt};
use futures_timer::Delay;
use gloo_worker::reactor::{reactor, ReactorScope};
use leptos::logging::{log, warn};
use minilm::{Cpu, MiniLM};
use r_ecipe_s_frontend::api::download;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum MiniLmWorkereComm {
    ModelPath(String),
    TextInput(String),
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct EncodeResponse(pub Vec<f32>);

async fn get_minilm(host: &str) -> Result<MiniLM<f32, Cpu>, Error> {
    let tokenizer_bytes = download(host, "tokenizer.json")
        .await
        .map_err(|err| Error::Msg(format!("{err}")))?;
    let model_bytes = download(host, "model.safetensors")
        .await
        .map_err(|err| Error::Msg(format!("{err}")))?;
    MiniLM::new(&tokenizer_bytes, &model_bytes).map_err(|err| Error::Msg(format!("{err}")))
}

/// A single threaded async mutex

#[derive(Debug)]
pub struct AsyncMutex<T> {
    locked: AtomicBool,
    inner: UnsafeCell<T>,
}

pub struct LockGuard<'a, T> {
    inner: &'a AsyncMutex<T>,
}

impl<T> AsyncMutex<T> {
    pub fn new(inner: T) -> Self {
        AsyncMutex {
            locked: AtomicBool::new(false),
            inner: UnsafeCell::new(inner),
        }
    }
    pub async fn lock(&self) -> LockGuard<'_, T> {
        while self.locked.fetch_or(true, Ordering::Relaxed) {
            Delay::new(Duration::from_millis(1)).await
        }
        LockGuard { inner: self }
    }
}

impl<T> Drop for LockGuard<'_, T> {
    fn drop(&mut self) {
        self.inner.locked.store(false, Ordering::Relaxed)
    }
}

impl<'a, T> Deref for LockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            self.inner
                .inner
                .get()
                .as_ref()
                .expect("Null pointer for unsafe cell")
        }
    }
}

impl<'a, T> DerefMut for LockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            self.inner
                .inner
                .get()
                .as_mut()
                .expect("Null pointer for unsafe cell")
        }
    }
}

impl<'a, T> AsRef<T> for LockGuard<'a, T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<'a, T> AsMut<T> for LockGuard<'a, T> {
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

#[reactor]
pub async fn EncodeOnDemand(mut scope: ReactorScope<MiniLmWorkereComm, EncodeResponse>) {
    log!("Starting minilm");
    let Some(MiniLmWorkereComm::ModelPath(path)) = scope.next().await else {
        warn!("Failed to get model path as first message");
        return;
    };
    let minilm_model = get_minilm(&path).await;
    log!("Started minilm");
    let minilm = match minilm_model {
        Ok(minilm_model) => minilm_model,
        Err(err) => {
            warn!("Failed to instantiate MiniLM: {err}");
            return;
        }
    };
    loop {
        match scope.next().await {
            Some(MiniLmWorkereComm::TextInput(request)) => {
                let Ok(encoded) = minilm.encode(&request) else {
                    warn!("Failed to encode data: {request}", request = request);
                    break;
                };

                if let Err(err) = scope.send(EncodeResponse(encoded)).await {
                    warn!("Failed to send encoded data. Cause: {err}");
                    break;
                }
            }

            Some(MiniLmWorkereComm::ModelPath(_)) => {
                warn!("Received model path after initialisation.");
                break;
            }
            _ => break,
        }
    }
}
#[derive(thiserror::Error, Debug, Clone, Deserialize, Serialize)]
pub enum Error {
    #[error("{0}")]
    Msg(String),
}
