use std::time::Duration;

use reqwest::header::{
    ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue, USER_AGENT,
};
use serde_json::{Value, json};
use subtle::ConstantTimeEq;
use url::Url;

use crate::connect::config::ConnectConfig;
use crate::connect::crypto::{Cipher, DpopKey, now};
use crate::connect::ecommerce::{OrderBatch, OrderResult};
use crate::connect::error::{ConnectError, ErrorCode};
use crate::connect::protocol::{
    AccessToken, CallbackQuery, PluginVersion, ShopName, ShopUrl, TokenCall, TokenResponse,
    error_code, map_token_error, new_nonce, start_url, token_thumbprint,
};
#[cfg(feature = "blocking")]
use crate::connect::records::BlockingConnectStorage;
use crate::connect::records::{ConnectStorage, Installation, Nonces, Token};
use crate::http::compose_user_agent;
use crate::reported::Reported;

const START_NONCE_LIFETIME: u64 = 3600;
const TIMEOUT: Duration = Duration::from_secs(10);
const DPOP: HeaderName = HeaderName::from_static("dpop");

struct Core {
    config: ConnectConfig,
    cipher: Cipher,
    host: Url,
    user_agent: String,
}

struct Wire {
    status: u16,
    dpop_nonce: Option<String>,
    body: Vec<u8>,
}

impl Core {
    fn new(config: ConnectConfig) -> Result<Self, ConnectError> {
        let host = config.host()?;
        Ok(Self {
            cipher: Cipher::new(&config.encryption_key),
            host,
            user_agent: compose_user_agent(&[])?,
            config,
        })
    }

    fn url(&self, path: &str) -> Result<Url, ConnectError> {
        self.host
            .join(path)
            .map_err(|_| ConnectError::UnexpectedResponse("the connect host is not a valid url"))
    }

    fn token_headers(&self, proof: &str, has_body: bool) -> Result<HeaderMap, ConnectError> {
        let mut headers = self.base_headers();
        headers.insert(DPOP, header(proof)?);
        if has_body {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }
        Ok(headers)
    }

    fn resource_headers(
        &self,
        token: &AccessToken,
        proof: &str,
    ) -> Result<HeaderMap, ConnectError> {
        let mut headers = self.base_headers();
        let mut authorization = header(&format!("DPoP {}", token.expose()))?;
        authorization.set_sensitive(true);
        headers.insert(AUTHORIZATION, authorization);
        headers.insert(DPOP, header(proof)?);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    fn base_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&self.user_agent)
                .unwrap_or_else(|_| HeaderValue::from_static("myparcel-rs")),
        );
        headers
    }

    fn wire(status: u16, nonce: Option<&HeaderValue>, body: Vec<u8>) -> Wire {
        Wire {
            status,
            dpop_nonce: nonce
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
            body,
        }
    }

    fn complete(
        &self,
        installation: &mut Installation,
        key: &DpopKey,
        response: &TokenResponse,
        challenge: Option<(&str, &str)>,
    ) -> Result<Token, ConnectError> {
        let _ = key;
        let issued = now();
        if let Some((htm, htu)) = challenge {
            installation.challenge_htm = Some(htm.to_owned());
            installation.challenge_htu = Some(htu.to_owned());
        }
        if let Some(id) = &response.connection_id {
            installation.connection_id = Some(id.clone());
        }
        installation.refresh_token_expires_at =
            Some(issued.saturating_add(response.refresh_expires_in));
        Ok(Token::new(
            self.cipher.encrypt_text(&response.access_token)?,
            issued.saturating_add(response.expires_in),
            response.scope.clone(),
        ))
    }
}

fn header(value: &str) -> Result<HeaderValue, ConnectError> {
    HeaderValue::from_str(value)
        .map_err(|_| ConnectError::UnexpectedResponse("a header value is not valid"))
}

fn invalid(reason: &str) -> ConnectError {
    ConnectError::InvalidCallback(reason.to_owned())
}

fn parse_results(wire: &Wire) -> Result<Vec<OrderResult>, ConnectError> {
    let body: Option<Value> = serde_json::from_slice(&wire.body).ok();
    if wire.status != 202 {
        return Err(ConnectError::Http {
            status: wire.status,
            code: error_code(body.as_ref()),
            body,
        });
    }
    match body {
        Some(Value::Array(items)) => Ok(items.iter().map(OrderResult::from_value).collect()),
        _ => Err(ConnectError::UnexpectedResponse(
            "the response is not a list of results",
        )),
    }
}

macro_rules! connect_flows {
    ($($async:ident)?; $($await:ident)?) => {
        $($async)? fn installation(&self) -> Result<Option<Installation>, ConnectError> {
            self.storage.load_installation() $(.$await)? .map_err(ConnectError::Storage)
        }

        $($async)? fn token(&self) -> Result<Option<Token>, ConnectError> {
            self.storage.load_token() $(.$await)? .map_err(ConnectError::Storage)
        }

        $($async)? fn nonces(&self) -> Result<Nonces, ConnectError> {
            Ok(self.storage.load_nonces() $(.$await)? .map_err(ConnectError::Storage)?.unwrap_or_else(Nonces::current))
        }

        $($async)? fn store_installation(&self, installation: &Installation) -> Result<(), ConnectError> {
            self.storage.save_installation(installation) $(.$await)? .map_err(ConnectError::Storage)
        }

        $($async)? fn store_token(&self, token: Option<&Token>) -> Result<(), ConnectError> {
            self.storage.save_token(token) $(.$await)? .map_err(ConnectError::Storage)
        }

        $($async)? fn store_nonces(&self, nonces: Option<&Nonces>) -> Result<(), ConnectError> {
            self.storage.save_nonces(nonces) $(.$await)? .map_err(ConnectError::Storage)
        }

        $($async)? fn installed(&self) -> Result<(Installation, DpopKey), ConnectError> {
            let installation = self.installation() $(.$await)? ?.ok_or(ConnectError::NotConnected)?;
            let key = self.core.cipher.decrypt_key(&installation.encrypted_private_key)?;
            Ok((installation, key))
        }

        /// Starts connecting a shop and returns the url the merchant has to be sent to.
        ///
        /// Redirect the browser there with a top-level navigation, not from an iframe or a
        /// server-side fetch. Protect the route with your platform's admin check. The key pair of
        /// the shop is generated on the first call and reused afterwards.
        pub $($async)? fn start(
            &self,
            shop_name: &ShopName,
            shop_url: &ShopUrl,
            version: Option<&PluginVersion>,
        ) -> Result<Url, ConnectError> {
            let key = match self.installation() $(.$await)? ? {
                Some(installation) => self.core.cipher.decrypt_key(&installation.encrypted_private_key)?,
                None => {
                    let key = DpopKey::generate()?;
                    let record = Installation::new(self.core.cipher.encrypt_key(&key)?);
                    self.store_installation(&record) $(.$await)? ?;
                    key
                }
            };
            let nonce = new_nonce()?;
            let mut nonces = self.nonces() $(.$await)? ?;
            nonces.start_nonce = Some(nonce.clone());
            nonces.start_nonce_expires_at = Some(now().saturating_add(START_NONCE_LIFETIME));
            self.store_nonces(Some(&nonces)) $(.$await)? ?;
            start_url(&self.core.config, &key, shop_name, shop_url, version, &nonce)
        }

        /// Completes the connection with the query MyParcel redirected the merchant back with.
        ///
        /// Pass the query as it is received: the nonce, code and url are validated here.
        pub $($async)? fn handle_callback(&self, query: &CallbackQuery) -> Result<(), ConnectError> {
            let (mut installation, key) = self.installed() $(.$await)? ?;
            let mut nonces = self.nonces() $(.$await)? ?;

            let presented = query.nonce().ok_or_else(|| invalid("`nonce` is missing"))?;
            let stored = nonces.start_nonce.clone().ok_or_else(|| invalid("no connection was started"))?;
            let fresh = nonces.start_nonce_expires_at.is_some_and(|expires| now() < expires);
            if !bool::from(presented.as_bytes().ct_eq(stored.as_bytes())) || !fresh {
                return Err(invalid("the nonce does not match or has expired"));
            }
            nonces.start_nonce = None;
            nonces.start_nonce_expires_at = None;
            self.store_nonces(Some(&nonces)) $(.$await)? ?;

            if let Some(error) = query.error() {
                return Err(ConnectError::from_callback(error));
            }
            let (code, htm, htu) = query.exchange_parameters()?;
            let response = self.token_call(TokenCall::Exchange, &key, htm, htu, Some(code)) $(.$await)? ?;

            let bound = token_thumbprint(&response.access_token).unwrap_or_default();
            if !bool::from(bound.as_bytes().ct_eq(key.thumbprint().as_bytes())) {
                return Err(ConnectError::KeyMismatch);
            }
            let token = self.core.complete(&mut installation, &key, &response, Some((htm, htu)))?;
            self.store_installation(&installation) $(.$await)? ?;
            self.store_token(Some(&token)) $(.$await)? ?;
            Ok(())
        }

        $($async)? fn token_call(
            &self,
            call: TokenCall,
            key: &DpopKey,
            htm: &str,
            htu: &str,
            code: Option<&str>,
        ) -> Result<TokenResponse, ConnectError> {
            let url = self.core.url(match call {
                TokenCall::Exchange => "connect/token",
                TokenCall::Refresh => "connect/refresh",
            })?;
            let body = code
                .map(|code| serde_json::to_vec(&json!({"code": code})))
                .transpose()
                .map_err(|_| ConnectError::Encryption)?;
            let mut nonces = self.nonces() $(.$await)? ?;
            let mut retried = false;
            loop {
                let proof = key.proof(htm, htu, nonces.dpop_nonce.as_deref(), None, now())?;
                let headers = self.core.token_headers(&proof, body.is_some())?;
                let wire = self.post(url.clone(), headers, body.clone()) $(.$await)? ?;

                if let Some(fresh) = &wire.dpop_nonce {
                    if nonces.dpop_nonce.as_ref() != Some(fresh) {
                        nonces.dpop_nonce = Some(fresh.clone());
                        self.store_nonces(Some(&nonces)) $(.$await)? ?;
                    }
                }
                if (200..300).contains(&wire.status) {
                    return TokenResponse::parse(&wire.body);
                }
                let parsed: Option<Value> = serde_json::from_slice(&wire.body).ok();
                let wants_nonce = error_code(parsed.as_ref())
                    .as_ref()
                    .and_then(Reported::known)
                    .is_some_and(|code| *code == ErrorCode::UseDpopNonce);
                if wants_nonce {
                    if retried || wire.dpop_nonce.is_none() {
                        return Err(ConnectError::NonceRequired);
                    }
                    retried = true;
                    continue;
                }
                return Err(map_token_error(call, wire.status, parsed));
            }
        }

        /// Gets a new access token. This normally happens by itself in
        /// [`access_token`](Self::access_token).
        pub $($async)? fn refresh(&self) -> Result<(), ConnectError> {
            let (mut installation, key) = self.installed() $(.$await)? ?;
            let (Some(htm), Some(htu)) = (installation.challenge_htm.clone(), installation.challenge_htu.clone())
            else {
                return Err(ConnectError::NotConnected);
            };
            match self.token_call(TokenCall::Refresh, &key, &htm, &htu, None) $(.$await)? {
                Ok(response) => {
                    let token = self.core.complete(&mut installation, &key, &response, None)?;
                    self.store_installation(&installation) $(.$await)? ?;
                    self.store_token(Some(&token)) $(.$await)? ?;
                    Ok(())
                }
                Err(ConnectError::ConnectionRevoked) => {
                    installation.refresh_token_expires_at = None;
                    self.store_token(None) $(.$await)? ?;
                    self.store_installation(&installation) $(.$await)? ?;
                    Err(ConnectError::ConnectionRevoked)
                }
                Err(error) => Err(error),
            }
        }

        /// Returns a valid access token, refreshing it when it expires within the leeway.
        pub $($async)? fn access_token(&self) -> Result<AccessToken, ConnectError> {
            let mut token = self.token() $(.$await)? ?.ok_or(ConnectError::NotConnected)?;
            let leeway = self.core.config.expiry_leeway.as_secs();
            if now().saturating_add(leeway) >= token.expires_at {
                self.refresh() $(.$await)? ?;
                token = self.token() $(.$await)? ?.ok_or(ConnectError::NotConnected)?;
            }
            Ok(AccessToken::new(self.core.cipher.decrypt_text(&token.encrypted_access_token)?))
        }

        /// Whether the shop has a usable connection according to storage. Any failure counts as
        /// not connected.
        pub $($async)? fn is_connected(&self) -> bool {
            let token = self.token() $(.$await)?;
            let installation = self.installation() $(.$await)?;
            matches!(token, Ok(Some(_)))
                && matches!(installation, Ok(Some(ref record)) if record.challenge_htm.is_some() && record.challenge_htu.is_some())
        }

        /// Forgets the connection but keeps the key pair, so the shop can connect again.
        ///
        /// This is local only: MyParcel has no revoke endpoint, and the old token stays valid
        /// until it expires.
        pub $($async)? fn disconnect(&self) -> Result<(), ConnectError> {
            if let Some(mut installation) = self.installation() $(.$await)? ? {
                installation.challenge_htm = None;
                installation.challenge_htu = None;
                installation.refresh_token_expires_at = None;
                self.store_installation(&installation) $(.$await)? ?;
            }
            self.store_token(None) $(.$await)? ?;
            self.store_nonces(None) $(.$await)? ?;
            Ok(())
        }

        /// Removes everything stored about the shop, for when the plugin is removed. The shop
        /// gets a new key pair when it connects again.
        pub $($async)? fn uninstall(&self) -> Result<(), ConnectError> {
            self.storage.clear() $(.$await)? .map_err(ConnectError::Storage)
        }

        /// Pushes orders to MyParcel.
        ///
        /// The result lists one outcome per order, in order. Some orders can be accepted while
        /// others are refused, so check every item.
        pub $($async)? fn push_orders(&self, orders: &OrderBatch) -> Result<Vec<OrderResult>, ConnectError> {
            let body = serde_json::to_vec(orders).map_err(|_| ConnectError::Encryption)?;
            let url = self.core.url("webhook/orders")?;
            let mut refreshed = false;
            loop {
                let token = self.access_token() $(.$await)? ?;
                let (_, key) = self.installed() $(.$await)? ?;
                let proof = key.proof("POST", url.as_str(), None, Some(token.expose()), now())?;
                let headers = self.core.resource_headers(&token, &proof)?;
                let wire = self.post(url.clone(), headers, Some(body.clone())) $(.$await)? ?;
                if wire.status == 401 && !refreshed {
                    refreshed = true;
                    self.refresh() $(.$await)? ?;
                    continue;
                }
                return parse_results(&wire);
            }
        }
    };
}

/// Connects a shop to MyParcel and pushes its orders, with asynchronous storage.
pub struct ConnectService<S> {
    core: Core,
    storage: S,
    http: reqwest::Client,
}

impl<S> std::fmt::Debug for ConnectService<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectService").finish_non_exhaustive()
    }
}

impl<S: ConnectStorage> ConnectService<S> {
    /// Creates the service. One service and one storage belong to one shop.
    pub fn new(config: ConnectConfig, storage: S) -> Result<Self, ConnectError> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(TIMEOUT)
            .build()?;
        Ok(Self {
            core: Core::new(config)?,
            storage,
            http,
        })
    }

    async fn post(
        &self,
        url: Url,
        headers: HeaderMap,
        body: Option<Vec<u8>>,
    ) -> Result<Wire, ConnectError> {
        let mut request = self.http.post(url).headers(headers);
        if let Some(body) = body {
            request = request.body(body);
        }
        let response = request.send().await?;
        let status = response.status().as_u16();
        let nonce = response.headers().get("dpop-nonce").cloned();
        let body = response.bytes().await?.to_vec();
        Ok(Core::wire(status, nonce.as_ref(), body))
    }

    connect_flows!(async; await);
}

/// Connects a shop to MyParcel and pushes its orders, with blocking storage.
#[cfg(feature = "blocking")]
pub struct BlockingConnectService<S> {
    core: Core,
    storage: S,
    http: reqwest::blocking::Client,
}

#[cfg(feature = "blocking")]
impl<S> std::fmt::Debug for BlockingConnectService<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockingConnectService")
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "blocking")]
impl<S: BlockingConnectStorage> BlockingConnectService<S> {
    /// Creates the service. One service and one storage belong to one shop.
    pub fn new(config: ConnectConfig, storage: S) -> Result<Self, ConnectError> {
        let http = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(TIMEOUT)
            .build()?;
        Ok(Self {
            core: Core::new(config)?,
            storage,
            http,
        })
    }

    fn post(
        &self,
        url: Url,
        headers: HeaderMap,
        body: Option<Vec<u8>>,
    ) -> Result<Wire, ConnectError> {
        let mut request = self.http.post(url).headers(headers);
        if let Some(body) = body {
            request = request.body(body);
        }
        let response = request.send()?;
        let status = response.status().as_u16();
        let nonce = response.headers().get("dpop-nonce").cloned();
        let body = response.bytes()?.to_vec();
        Ok(Core::wire(status, nonce.as_ref(), body))
    }

    connect_flows!(; );
}
