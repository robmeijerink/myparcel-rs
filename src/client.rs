use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use url::Url;

use crate::auth::ApiKey;
use crate::error::{Result, ValidationError};
use crate::http::{
    CONNECT_TIMEOUT, CORE_URL, DEFAULT_TIMEOUT, IAM_URL, Operation, Response, Transport,
    compose_user_agent, user_agent_token, validate_base_url,
};

/// Asynchronous client for the MyParcel API.
///
/// Cloning is cheap; clones share the connection pool.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

struct Inner {
    http: reqwest::Client,
    transport: Transport,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Client {
    /// Creates a client with default settings.
    pub fn new(api_key: ApiKey) -> Result<Self> {
        Self::builder(api_key).build()
    }

    /// Creates a client with the key from `API_KEY`, `API_KEY_NL` or `API_KEY_BE`.
    pub fn from_env() -> Result<Self> {
        Self::new(ApiKey::from_env()?)
    }

    /// Returns a builder for customising the client.
    pub fn builder(api_key: ApiKey) -> ClientBuilder {
        ClientBuilder::new(api_key)
    }

    /// Operations on shipments.
    pub fn shipments(&self) -> crate::shipments::Shipments<'_> {
        crate::shipments::Shipments { client: self }
    }

    /// Operations on labels.
    pub fn labels(&self) -> crate::labels::Labels<'_> {
        crate::labels::Labels { client: self }
    }

    /// Operations on track and trace data.
    pub fn track_traces(&self) -> crate::track_trace::TrackTraces<'_> {
        crate::track_trace::TrackTraces { client: self }
    }

    /// Operations on return shipments.
    pub fn returns(&self) -> crate::returns::Returns<'_> {
        crate::returns::Returns { client: self }
    }

    /// Operations on webhook subscriptions.
    pub fn webhooks(&self) -> crate::webhooks::Webhooks<'_> {
        crate::webhooks::Webhooks { client: self }
    }

    /// Operations on carrier capabilities.
    pub fn capabilities(&self) -> crate::capabilities::Capabilities<'_> {
        crate::capabilities::Capabilities { client: self }
    }

    /// Operations on the account behind the API key.
    pub fn account(&self) -> crate::account::Account<'_> {
        crate::account::Account { client: self }
    }

    pub(crate) async fn execute<O: Operation>(&self, operation: O) -> Result<O::Output> {
        let transport = &self.inner.transport;
        let request = operation.request(&transport.context())?;
        let url = transport.url(&request)?;
        let mut builder = self
            .inner
            .http
            .request(request.method.clone(), url.clone())
            .headers(transport.headers(&request)?);
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        let response = builder.send().await?;
        let status = response.status().as_u16();
        let body = response.bytes().await?.to_vec();
        operation.parse(Response { status, url, body })
    }
}

/// Builder for [`Client`] and `blocking::Client`.
pub struct ClientBuilder {
    api_key: ApiKey,
    base_url: Url,
    iam_base_url: Url,
    user_agent: Vec<(String, String)>,
    timeout: Duration,
    connect_timeout: Duration,
    http: Option<reqwest::Client>,
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("base_url", &self.base_url.as_str())
            .field("iam_base_url", &self.iam_base_url.as_str())
            .finish_non_exhaustive()
    }
}

impl ClientBuilder {
    pub(crate) fn new(api_key: ApiKey) -> Self {
        Self {
            api_key,
            base_url: Url::parse(CORE_URL).expect("constant url is valid"),
            iam_base_url: Url::parse(IAM_URL).expect("constant url is valid"),
            user_agent: Vec::new(),
            timeout: DEFAULT_TIMEOUT,
            connect_timeout: CONNECT_TIMEOUT,
            http: None,
        }
    }

    /// Overrides the base url of the core API. It must use `https`; `http` is only accepted for
    /// loopback hosts.
    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = url;
        self
    }

    /// Overrides the base url of the IAM API, with the same rules as [`ClientBuilder::base_url`].
    pub fn iam_base_url(mut self, url: Url) -> Self {
        self.iam_base_url = url;
        self
    }

    /// Adds a `platform/version` pair in front of the crate's own user agent. MyParcel asks
    /// integrations to identify their CMS this way.
    pub fn user_agent(mut self, platform: impl Into<String>, version: impl Into<String>) -> Self {
        self.user_agent.push((platform.into(), version.into()));
        self
    }

    /// Sets the timeout of a whole request. The default is 60 seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the timeout for establishing a connection. The default is 10 seconds.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Uses an existing HTTP client. Its settings apply as they are, so make sure it does not
    /// follow redirects.
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    fn transport(
        self,
    ) -> std::result::Result<
        (Transport, Option<reqwest::Client>, Duration, Duration),
        ValidationError,
    > {
        validate_base_url("base url", &self.base_url)?;
        validate_base_url("iam base url", &self.iam_base_url)?;
        for (platform, version) in &self.user_agent {
            user_agent_token("user agent platform", platform)?;
            user_agent_token("user agent version", version)?;
        }
        let user_agent = compose_user_agent(&self.user_agent)?;
        let transport = Transport::new(self.api_key, self.base_url, self.iam_base_url, user_agent);
        Ok((transport, self.http, self.timeout, self.connect_timeout))
    }

    #[cfg(feature = "blocking")]
    pub(crate) fn into_blocking_parts(
        self,
    ) -> std::result::Result<(Transport, Duration, Duration), ValidationError> {
        let (transport, _, timeout, connect_timeout) = self.transport()?;
        Ok((transport, timeout, connect_timeout))
    }

    /// Builds the asynchronous client.
    pub fn build(self) -> Result<Client> {
        let (transport, http, timeout, connect_timeout) = self.transport()?;
        let http = match http {
            Some(http) => http,
            None => reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(timeout)
                .connect_timeout(connect_timeout)
                .build()?,
        };
        Ok(Client {
            inner: Arc::new(Inner { http, transport }),
        })
    }
}
