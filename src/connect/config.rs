use std::fmt;
use std::time::Duration;

use url::Url;
use zeroize::Zeroizing;

use crate::connect::ConnectError;
use crate::error::ValidationError;
use crate::http::validate_base_url;

/// The path MyParcel redirects the merchant back to, below the shop's url.
pub const CALLBACK_PATH: &str = "/myparcel-connect/callback";

string_enum! {
    /// The e-commerce platform a shop runs on.
    pub enum ConnectPlatform {
        /// Shopify.
        Shopify = "shopify",
    }
}

impl ConnectPlatform {
    pub(crate) const fn config_name(self) -> &'static str {
        match self {
            Self::Shopify => "SHOPIFY",
        }
    }
}

string_enum! {
    /// What a connection may be used for.
    pub enum ConnectScope {
        /// Pushing orders.
        Integration = "integration",
    }
}

/// The secret that protects the private key and the token in storage.
///
/// Keep it apart from the storage itself, for example in a secret manager.
#[derive(Clone)]
pub struct EncryptionKey(Zeroizing<String>);

impl EncryptionKey {
    /// Wraps a secret, rejecting an empty one.
    pub fn new(secret: impl Into<String>) -> Result<Self, ValidationError> {
        let secret = Zeroizing::new(secret.into());
        if secret.is_empty() {
            return Err(ValidationError::new("encryption key", "must not be empty"));
        }
        Ok(Self(secret))
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EncryptionKey([redacted])")
    }
}

/// Settings of a [`ConnectService`](crate::connect::ConnectService).
#[derive(Debug, Clone)]
pub struct ConnectConfig {
    pub(crate) platform: ConnectPlatform,
    pub(crate) encryption_key: EncryptionKey,
    pub(crate) expiry_leeway: Duration,
    pub(crate) scopes: Vec<ConnectScope>,
    pub(crate) acceptance: bool,
    pub(crate) base_url: Option<Url>,
}

impl ConnectConfig {
    /// A configuration with the default scope and a refresh leeway of 30 seconds.
    pub fn new(platform: ConnectPlatform, encryption_key: EncryptionKey) -> Self {
        Self {
            platform,
            encryption_key,
            expiry_leeway: Duration::from_secs(30),
            scopes: vec![ConnectScope::Integration],
            acceptance: false,
            base_url: None,
        }
    }

    /// Refreshes the access token this long before it expires.
    pub fn with_expiry_leeway(mut self, leeway: Duration) -> Self {
        self.expiry_leeway = leeway;
        self
    }

    /// Sets the scopes. At least one is required; duplicates are removed.
    pub fn with_scopes(mut self, scopes: Vec<ConnectScope>) -> Result<Self, ValidationError> {
        if scopes.is_empty() {
            return Err(ValidationError::new(
                "scopes",
                "at least one scope is required",
            ));
        }
        let mut unique = Vec::new();
        for scope in scopes {
            if !unique.contains(&scope) {
                unique.push(scope);
            }
        }
        self.scopes = unique;
        Ok(self)
    }

    /// Uses the acceptance environment of MyParcel.
    pub fn with_acceptance(mut self, acceptance: bool) -> Self {
        self.acceptance = acceptance;
        self
    }

    /// Overrides the host. It must use `https`; `http` is only accepted for loopback hosts.
    pub fn with_base_url(mut self, url: Url) -> Result<Self, ValidationError> {
        validate_base_url("connect base url", &url)?;
        self.base_url = Some(url);
        Ok(self)
    }

    pub(crate) fn host(&self) -> Result<Url, ConnectError> {
        if let Some(url) = &self.base_url {
            return Ok(url.clone());
        }
        let environment = if self.acceptance { ".acceptance" } else { "" };
        let text = format!(
            "https://{}.ecommerce.api{environment}.myparcel.nl/",
            self.platform
        );
        Url::parse(&text)
            .map_err(|_| ConnectError::UnexpectedResponse("the connect host is not a valid url"))
    }

    pub(crate) fn scope_value(&self) -> String {
        self.scopes
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}
