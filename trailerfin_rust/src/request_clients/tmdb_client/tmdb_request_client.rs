use std::borrow::Cow;
use serde::Serialize;
use anyhow::{anyhow, Result};
use crate::request_clients::rate_limited_client::{Executor, RateLimitedClient};
use crate::request_clients::request_errors::error::Error;
use crate::request_clients::tmdb_client::external_ids_endpoints::ExternalIdsService;
use crate::request_clients::tmdb_client::videos_endpoints::VideosService;

#[derive(Debug)]
pub struct TmdbRequestClient(
    pub Client<RateLimitedClient>
);

const BASE_URL: &str = "https://api.themoviedb.org/3";

pub struct ClientBuilder<E: Executor> {
    base_url: Cow<'static, str>,
    executor: Option<E>,
    api_key: Option<String>,
}

impl<E: Executor> Default for ClientBuilder<E> {
    fn default() -> Self {
        Self {
            base_url: Cow::Borrowed(BASE_URL),
            executor: None,
            api_key: None,
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

    pub fn with_api_key(mut self, value: String) -> Self {
        self.api_key = Some(value);
        self
    }

    pub fn set_api_key(mut self, value: String) {
        self.api_key = Some(value);
    }

    pub fn build(self) -> Result<Client<E>> {
        let base_url = self.base_url;
        let executor = self.executor.ok_or_else(|| anyhow!("missing executor"))?;
        let api_key = self.api_key.ok_or_else(|| anyhow!("missing api key"))?;

        Ok(Client {
            executor,
            base_url,
            api_key,
        })
    }
}

#[derive(Serialize)]
struct WithApiKey<V> {
    api_key: String,
    #[serde(flatten)]
    inner: V,
}

pub struct Client<E> {
    executor: E,
    base_url: Cow<'static, str>,
    api_key: String,
}

impl<E: std::fmt::Debug> std::fmt::Debug for Client<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Client))
            .field("executor", &self.executor)
            .field("base_url", &self.base_url)
            .field("api_key", &"REDACTED")
            .finish()
    }
}

impl<E: Executor> Client<E> {
    pub fn builder() -> ClientBuilder<E> {
        ClientBuilder::default()
    }
}

impl TmdbRequestClient {
    pub fn base_url(&self) -> &str {
        &self.0.base_url
    }

    pub async fn execute<T, P>(&self, path: &str, params: P) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
        P: serde::Serialize + Send + Sync + 'static,
    {
        let url = format!("{}{}", self.0.base_url, path);
        self.0.executor
            .execute(
                &url,
                WithApiKey {
                    api_key: self.0.api_key.clone(),
                    inner: params,
                },
            )
            .await
    }

    pub fn external_ids(&self) -> ExternalIdsService<'_> {
        ExternalIdsService { client: self }
    }

    pub fn videos(&self) -> VideosService<'_> {
        VideosService { client: self }
    }
}