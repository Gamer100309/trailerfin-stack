use std::borrow::Cow;
use anyhow::{anyhow, Result};
use serde::de::DeserializeOwned;
use crate::request_clients::rate_limited_client::{Executor, RateLimitedClient};
use crate::request_clients::request_errors::error::Error;

#[derive(Debug)]
pub struct ImdbRequestClient(
    pub Client<RateLimitedClient>
);

const BASE_URL: &str = "https://www.imdb.com";

pub struct ClientBuilder<E: Executor> {
    base_url: Cow<'static, str>,
    executor: Option<E>
}

impl<E: Executor> Default for ClientBuilder<E> {
    fn default() -> Self {
        Self {
            base_url: Cow::Borrowed(BASE_URL),
            executor: None,
        }
    }
}

impl<E: Executor> ClientBuilder<E> {
    pub fn with_base_url<U: Into<Cow<'static, str>>>(mut self, value: U) -> Self {
        self.base_url = value.into();
        self
    }

    pub fn set_base_url<U: Into<Cow<'static, str>>>(&mut self, value: U) {
        self.base_url = value.into();
    }

    pub fn with_executor(mut self, executor: E) -> Self {
        self.executor = Some(executor);
        self
    }

    pub fn set_executor(mut self, executor: E) {
        self.executor = Some(executor);
    }

    pub fn build(self) -> Result<Client<E>> {
        let base_url = self.base_url;
        let executor = self.executor.ok_or_else(|| anyhow!("missing executor"))?;

        Ok(Client {
            executor,
            base_url,
        })
    }
}

pub struct Client<E> {
    executor: E,
    base_url: Cow<'static, str>,
}

impl<E: std::fmt::Debug> std::fmt::Debug for Client<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Client))
            .field("executor", &self.executor)
            .field("base_url", &self.base_url)
            .finish()
    }
}

impl<E: Executor> Client<E> {
    pub fn builder() -> ClientBuilder<E> {
        ClientBuilder::default()
    }
}


impl ImdbRequestClient {
    pub fn base_url(&self) -> &str {
        &self.0.base_url
    }

    pub async fn execute<T>(&self, _: &str) -> Result<T, Error>
    where
        T: DeserializeOwned + Send + 'static,
    {
        Err(Error::UnsupportedOperation("ImdbClient does not support generic execute() calls".into()))

    }

    pub async fn get_raw(&self, path: &str) -> Result<reqwest::Response, Error> {
        let url = format!("{}{}", self.0.base_url, path);
        self.0.executor
            .execute_raw(&url)
            .await
    }
}