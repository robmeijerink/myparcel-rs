use std::fmt;

use serde::Deserialize;
use serde_json::{Value, json};
use url::Url;
use zeroize::Zeroizing;

use crate::connect::config::ConnectConfig;
use crate::connect::crypto::{DpopKey, base64url, base64url_decode};
use crate::connect::error::{ConnectError, ErrorCode};
use crate::error::ValidationError;
use crate::reported::Reported;

const MAX_CONFIG_LEN: usize = 1024;
const MAX_SHOP_URL_LEN: usize = 128;

fn is_invisible(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{ad}' | '\u{200b}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{2064}' | '\u{feff}')
}

bounded_string! {
    /// The name of a shop, 1 to 64 characters without control or invisible formatting characters.
    ShopName, "shop name", 1, 64,
    |value| value.chars().any(is_invisible).then_some("must not contain control characters")
}

bounded_string! {
    /// The version of the plugin that connects the shop, 1 to 256 characters.
    PluginVersion, "plugin version", 1, 256
}

/// The base url of a shop.
///
/// Take it from your own configuration, never from a request. It has to use `https`, be at most
/// 128 bytes long and carry no query, fragment or credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopUrl(String);

impl ShopUrl {
    /// Validates a shop url.
    pub fn new(url: &Url) -> Result<Self, ValidationError> {
        let field = "shop url";
        if url.scheme() != "https" || url.host_str().is_none() {
            return Err(ValidationError::new(
                field,
                "must be an https url with a host",
            ));
        }
        if url.query().is_some() || url.fragment().is_some() {
            return Err(ValidationError::new(
                field,
                "must not have a query or fragment",
            ));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(ValidationError::new(field, "must not contain credentials"));
        }
        let text = url.as_str().trim_end_matches('/').to_owned();
        if text.len() > MAX_SHOP_URL_LEN {
            return Err(ValidationError::new(
                field,
                format!("is longer than {MAX_SHOP_URL_LEN} bytes"),
            ));
        }
        Ok(Self(text))
    }

    /// The url without a trailing slash.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for ShopUrl {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url =
            Url::parse(s).map_err(|error| ValidationError::new("shop url", error.to_string()))?;
        Self::new(&url)
    }
}

/// An access token. It is wiped from memory on drop and never shown by `Debug`.
pub struct AccessToken(Zeroizing<String>);

impl AccessToken {
    pub(crate) fn new(token: Zeroizing<String>) -> Self {
        Self(token)
    }

    /// The token, for use in an `Authorization: DPoP` header.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccessToken([redacted])")
    }
}

/// The query MyParcel redirects the merchant back with.
///
/// `Debug` shows which parameters are present, not their values: the code is a credential until
/// it is exchanged.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct CallbackQuery {
    nonce: Option<String>,
    code: Option<String>,
    htm: Option<String>,
    htu: Option<String>,
    error: Option<String>,
}

impl fmt::Debug for CallbackQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallbackQuery")
            .field("nonce", &self.nonce.is_some())
            .field("code", &self.code.is_some())
            .field("htm", &self.htm)
            .field("htu", &self.htu)
            .field("error", &self.error)
            .finish()
    }
}

impl CallbackQuery {
    /// Reads the callback parameters from key-value pairs. Other parameters are ignored.
    pub fn from_pairs<K: AsRef<str>, V: Into<String>>(
        pairs: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        let mut query = Self::default();
        for (name, value) in pairs {
            let value = value.into();
            match name.as_ref() {
                "nonce" => query.nonce = Some(value),
                "code" => query.code = Some(value),
                "htm" => query.htm = Some(value),
                "htu" => query.htu = Some(value),
                "error" => query.error = Some(value),
                _ => {}
            }
        }
        query
    }

    /// Reads the callback parameters from a raw query string, without the leading `?`.
    pub fn from_query_string(query: &str) -> Self {
        Self::from_pairs(url::form_urlencoded::parse(query.as_bytes()))
    }

    pub(crate) fn nonce(&self) -> Option<&str> {
        self.nonce.as_deref()
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref().filter(|error| !error.is_empty())
    }

    pub(crate) fn exchange_parameters(&self) -> Result<(&str, &str, &str), ConnectError> {
        let code = required(&self.code, "code")?;
        let htm = required(&self.htm, "htm")?;
        let htu = required(&self.htu, "htu")?;
        if code.len() != 43
            || !code
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(ConnectError::InvalidCallback(
                "`code` has an unexpected format".into(),
            ));
        }
        if htm.len() > 32 || !htm.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(ConnectError::InvalidCallback(
                "`htm` has an unexpected format".into(),
            ));
        }
        let parsed = Url::parse(htu)
            .map_err(|_| ConnectError::InvalidCallback("`htu` is not a url".into()))?;
        if parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(ConnectError::InvalidCallback(
                "`htu` must be an https url without query or fragment".into(),
            ));
        }
        Ok((code, htm, htu))
    }
}

fn required<'a>(value: &'a Option<String>, name: &str) -> Result<&'a str, ConnectError> {
    value
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ConnectError::InvalidCallback(format!("`{name}` is missing")))
}

pub(crate) fn new_nonce() -> Result<String, ConnectError> {
    Ok(base64url(&crate::connect::crypto::random_bytes::<32>()?))
}

pub(crate) fn start_url(
    config: &ConnectConfig,
    key: &DpopKey,
    name: &ShopName,
    shop_url: &ShopUrl,
    version: Option<&PluginVersion>,
    nonce: &str,
) -> Result<Url, ConnectError> {
    let mut start = json!({
        "platform": config.platform.config_name(),
        "shopName": name.as_str(),
        "shopUrl": shop_url.as_str(),
    });
    if let Some(version) = version {
        start["version"] = Value::String(version.as_str().to_owned());
    }
    let encoded = base64url(&serde_json::to_vec(&start).map_err(|_| ConnectError::Encryption)?);
    if encoded.len() > MAX_CONFIG_LEN {
        return Err(ValidationError::new(
            "connect config",
            format!("is longer than {MAX_CONFIG_LEN} bytes"),
        )
        .into());
    }
    let mut url = config
        .host()?
        .join("connect/start")
        .map_err(|_| ConnectError::UnexpectedResponse("the connect host is not a valid url"))?;
    url.query_pairs_mut()
        .append_pair("jkt", &key.thumbprint())
        .append_pair("config", &encoded)
        .append_pair("nonce", nonce)
        .append_pair("scope", &config.scope_value());
    Ok(url)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenResponse {
    pub access_token: Zeroizing<String>,
    pub connection_id: Option<String>,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_expires_in: u64,
    pub scope: String,
}

impl TokenResponse {
    pub fn parse(body: &[u8]) -> Result<Self, ConnectError> {
        let response: Self = serde_json::from_slice(body).map_err(|_| {
            ConnectError::UnexpectedResponse("the token response could not be read")
        })?;
        if response.token_type != "DPoP" {
            return Err(ConnectError::UnexpectedResponse(
                "the token is not a DPoP token",
            ));
        }
        if response.expires_in < 1 || response.refresh_expires_in < 1 {
            return Err(ConnectError::UnexpectedResponse(
                "the token expires immediately",
            ));
        }
        Ok(response)
    }
}

pub(crate) fn token_thumbprint(access_token: &str) -> Option<String> {
    let payload = access_token.split('.').nth(1)?;
    let claims: Value = serde_json::from_slice(&base64url_decode(payload)?).ok()?;
    claims.get("cnf")?.get("jkt")?.as_str().map(str::to_owned)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenCall {
    Exchange,
    Refresh,
}

pub(crate) fn error_code(body: Option<&Value>) -> Option<Reported<ErrorCode>> {
    let code = body?.get("error")?.clone();
    serde_json::from_value(code).ok()
}

pub(crate) fn map_token_error(call: TokenCall, status: u16, body: Option<Value>) -> ConnectError {
    let code = error_code(body.as_ref());
    match (call, code.as_ref().and_then(Reported::known)) {
        (TokenCall::Exchange, Some(ErrorCode::InvalidGrant)) => ConnectError::SessionExpired,
        (TokenCall::Refresh, Some(ErrorCode::InvalidGrant)) => ConnectError::ConnectionRevoked,
        (_, Some(ErrorCode::InvalidSalesChannel)) => ConnectError::InvalidSalesChannel,
        (TokenCall::Exchange, Some(ErrorCode::InvalidCode)) => {
            ConnectError::InvalidCallback("MyParcel did not recognise the code".into())
        }
        (_, Some(ErrorCode::ServerError)) => ConnectError::ServerError(None),
        _ => ConnectError::Http { status, code, body },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connect::config::{ConnectPlatform, EncryptionKey};

    fn config() -> ConnectConfig {
        ConnectConfig::new(ConnectPlatform::Shopify, EncryptionKey::new("k").unwrap())
    }

    #[test]
    fn shop_names_reject_control_characters_and_overlong_values() {
        assert!(ShopName::new("My shop").is_ok());
        assert!(ShopName::new("My\nshop").is_err());
        assert!(ShopName::new("zero\u{200b}width").is_err());
        assert!(ShopName::new("x".repeat(65)).is_err());
        assert!(ShopName::new("").is_err());
    }

    #[test]
    fn shop_urls_must_be_plain_https() {
        assert_eq!(
            "https://shop.example".parse::<ShopUrl>().unwrap().as_str(),
            "https://shop.example"
        );
        assert_eq!(
            "https://shop.example/sub/"
                .parse::<ShopUrl>()
                .unwrap()
                .as_str(),
            "https://shop.example/sub"
        );
        assert!("http://shop.example".parse::<ShopUrl>().is_err());
        assert!("https://shop.example/?a=1".parse::<ShopUrl>().is_err());
        assert!("https://shop.example/#a".parse::<ShopUrl>().is_err());
        assert!("https://user@shop.example".parse::<ShopUrl>().is_err());
        assert!(
            format!("https://{}.example", "a".repeat(130))
                .parse::<ShopUrl>()
                .is_err()
        );
    }

    #[test]
    fn the_start_url_carries_the_key_thumbprint_config_nonce_and_scope() {
        let key = DpopKey::generate().unwrap();
        let url = start_url(
            &config(),
            &key,
            &ShopName::new("Shop").unwrap(),
            &"https://shop.example".parse().unwrap(),
            Some(&PluginVersion::new("1.2.3").unwrap()),
            "nonce-1",
        )
        .unwrap();
        assert_eq!(url.host_str(), Some("shopify.ecommerce.api.myparcel.nl"));
        assert_eq!(url.path(), "/connect/start");
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query["jkt"], key.thumbprint());
        assert_eq!(query["nonce"], "nonce-1");
        assert_eq!(query["scope"], "integration");
        let config: Value =
            serde_json::from_slice(&base64url_decode(&query["config"]).unwrap()).unwrap();
        assert_eq!(
            config,
            json!({"platform": "SHOPIFY", "shopName": "Shop", "shopUrl": "https://shop.example", "version": "1.2.3"})
        );
    }

    #[test]
    fn callback_parameters_are_checked() {
        let code = "A".repeat(43);
        let ok = CallbackQuery::from_query_string(&format!(
            "code={code}&htm=POST&htu=https%3A%2F%2Fx.test%2Ftoken&nonce=n"
        ));
        assert_eq!(
            ok.exchange_parameters().unwrap(),
            (code.as_str(), "POST", "https://x.test/token")
        );
        assert_eq!(ok.nonce(), Some("n"));

        for bad in [
            "htm=POST&htu=https://x.test/".to_owned(),
            "code=short&htm=POST&htu=https://x.test/".to_owned(),
            format!("code={code}&htm=POST&htu=http://x.test/"),
            format!("code={code}&htm=POST&htu=https://x.test/?a=b"),
            format!("code={code}&htm=POST&htu=not-a-url"),
        ] {
            assert!(
                CallbackQuery::from_query_string(&bad)
                    .exchange_parameters()
                    .is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn debug_hides_the_code_and_the_nonce() {
        let query = CallbackQuery::from_query_string("code=SECRETCODE&nonce=SECRETNONCE&htm=POST");
        let shown = format!("{query:?}");
        assert!(
            !shown.contains("SECRETCODE") && !shown.contains("SECRETNONCE"),
            "{shown}"
        );
        assert!(shown.contains("POST"));
    }

    #[test]
    fn token_responses_are_validated() {
        let ok = br#"{"accessToken":"a.b.c","connectionId":"c","tokenType":"DPoP","expiresIn":300,"refreshExpiresIn":3600,"scope":"integration"}"#;
        assert_eq!(TokenResponse::parse(ok).unwrap().expires_in, 300);
        let bearer = br#"{"accessToken":"a","tokenType":"Bearer","expiresIn":300,"refreshExpiresIn":3600,"scope":"s"}"#;
        assert!(TokenResponse::parse(bearer).is_err());
        let instant = br#"{"accessToken":"a","tokenType":"DPoP","expiresIn":0,"refreshExpiresIn":3600,"scope":"s"}"#;
        assert!(TokenResponse::parse(instant).is_err());
    }

    #[test]
    fn the_token_thumbprint_is_read_from_the_cnf_claim() {
        let payload = base64url(br#"{"cnf":{"jkt":"abc"}}"#);
        assert_eq!(
            token_thumbprint(&format!("h.{payload}.s")).as_deref(),
            Some("abc")
        );
        assert_eq!(token_thumbprint("nonsense"), None);
    }

    #[test]
    fn token_errors_are_translated() {
        let body = |code: &str| Some(json!({"error": code}));
        assert!(matches!(
            map_token_error(TokenCall::Exchange, 400, body("invalid_grant")),
            ConnectError::SessionExpired
        ));
        assert!(matches!(
            map_token_error(TokenCall::Refresh, 400, body("invalid_grant")),
            ConnectError::ConnectionRevoked
        ));
        assert!(matches!(
            map_token_error(TokenCall::Exchange, 400, body("invalid_code")),
            ConnectError::InvalidCallback(_)
        ));
        assert!(matches!(
            map_token_error(TokenCall::Refresh, 500, body("server_error")),
            ConnectError::ServerError(None)
        ));
        assert!(matches!(
            map_token_error(TokenCall::Refresh, 401, body("invalid_dpop_proof")),
            ConnectError::Http { status: 401, .. }
        ));
    }
}
