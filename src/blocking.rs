//! A blocking client with the same operations as the asynchronous [`Client`](crate::Client).
//!
//! Do not use it from inside an asynchronous runtime: it panics there, as `reqwest::blocking`
//! does.

use std::fmt;
use std::sync::Arc;

use crate::auth::ApiKey;
use crate::client::ClientBuilder;
use crate::error::Result;
use crate::http::{Operation, Response, Transport};

/// Blocking client for the MyParcel API.
///
/// Cloning is cheap; clones share the connection pool.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

struct Inner {
    http: reqwest::blocking::Client,
    transport: Transport,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl ClientBuilder {
    /// Builds the blocking client. A client passed to [`ClientBuilder::http_client`] is ignored.
    pub fn build_blocking(self) -> Result<Client> {
        let (transport, timeout, connect_timeout) = self.into_blocking_parts()?;
        let http = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(timeout)
            .connect_timeout(connect_timeout)
            .build()?;
        Ok(Client {
            inner: Arc::new(Inner { http, transport }),
        })
    }
}

impl Client {
    /// Creates a client with default settings.
    pub fn new(api_key: ApiKey) -> Result<Self> {
        Self::builder(api_key).build_blocking()
    }

    /// Creates a client with the key from `API_KEY`, `API_KEY_NL` or `API_KEY_BE`.
    pub fn from_env() -> Result<Self> {
        Self::new(ApiKey::from_env()?)
    }

    /// Returns a builder for customising the client. Finish it with
    /// [`ClientBuilder::build_blocking`].
    pub fn builder(api_key: ApiKey) -> ClientBuilder {
        ClientBuilder::new(api_key)
    }

    /// Operations on the account behind the API key.
    pub fn account(&self) -> Account<'_> {
        Account { client: self }
    }

    /// Operations on carrier capabilities.
    pub fn capabilities(&self) -> Capabilities<'_> {
        Capabilities { client: self }
    }

    /// Operations on labels.
    pub fn labels(&self) -> Labels<'_> {
        Labels { client: self }
    }

    /// Operations on return shipments.
    pub fn returns(&self) -> Returns<'_> {
        Returns { client: self }
    }

    /// Operations on shipments.
    pub fn shipments(&self) -> Shipments<'_> {
        Shipments { client: self }
    }

    /// Operations on track and trace data.
    pub fn track_traces(&self) -> TrackTraces<'_> {
        TrackTraces { client: self }
    }

    /// Operations on webhook subscriptions.
    pub fn webhooks(&self) -> Webhooks<'_> {
        Webhooks { client: self }
    }

    pub(crate) fn execute<O: Operation>(&self, operation: O) -> Result<O::Output> {
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
        let response = builder.send()?;
        let status = response.status().as_u16();
        let body = response.bytes()?.to_vec();
        operation.parse(Response { status, url, body })
    }
}

macro_rules! blocking_service {
    ($(#[$meta:meta])* $name:ident, $methods:path) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'a> {
            pub(crate) client: &'a Client,
        }

        impl $name<'_> {
            $methods!(;);
        }
    };
}

blocking_service!(
    /// Operations on the account.
    Account,
    crate::account::account_methods
);
blocking_service!(
    /// Operations on carrier capabilities.
    Capabilities,
    crate::capabilities::capabilities_methods
);
blocking_service!(
    /// Operations on labels.
    Labels,
    crate::labels::labels_methods
);
blocking_service!(
    /// Operations on return shipments.
    Returns,
    crate::returns::returns_methods
);
blocking_service!(
    /// Operations on shipments.
    Shipments,
    crate::shipments::shipments_methods
);
blocking_service!(
    /// Operations on track and trace data.
    TrackTraces,
    crate::track_trace::track_trace_methods
);
blocking_service!(
    /// Operations on webhook subscriptions.
    Webhooks,
    crate::webhooks::webhooks_methods
);
