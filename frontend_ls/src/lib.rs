use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use futures::{SinkExt, StreamExt};
use futures_timer::Delay;
use leptos::logging::{log, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum MiniLmWorkereComm {
    ModelData {
        tokenizer_bytes: Vec<u8>,
        weights_bytes: Vec<u8>,
    },
    TextInput(String),
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct EncodeResponse(pub Vec<f32>);

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

#[derive(thiserror::Error, Debug, Clone, Deserialize, Serialize)]
pub enum Error {
    #[error("{0}")]
    Msg(String),
}
