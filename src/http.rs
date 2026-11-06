use std::borrow::Cow;
use std::time::Duration;

use reqwest::Method;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde::de::DeserializeOwned;
use url::Url;

use crate::auth::ApiKey;
use crate::error::{ApiError, Error, Result, ValidationError, truncate};

pub(crate) const CORE_URL: &str = "https://api.myparcel.nl/";
pub(crate) const IAM_URL: &str = "https://iam.api.myparcel.nl/";
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_USER_AGENT_LEN: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Host {
    Core,
    Iam,
}

pub(crate) struct Request {
    pub method: Method,
    pub host: Host,
    pub segments: Vec<String>,
    pub query: Vec<(&'static str, String)>,
    pub accept: Cow<'static, str>,
    pub content_type: Option<&'static str>,
    pub body: Option<Vec<u8>>,
}

impl Request {
    pub fn new(
        method: Method,
        host: Host,
        segments: Vec<String>,
        accept: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            method,
            host,
            segments,
            query: Vec::new(),
            accept: accept.into(),
            content_type: None,
            body: None,
        }
    }

    pub fn query(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.query.push((name, value.into()));
        self
    }

    pub fn json_body<T: serde::Serialize>(
        mut self,
        content_type: &'static str,
        body: &T,
    ) -> Result<Self> {
        let bytes = serde_json::to_vec(body).map_err(|source| Error::Decode {
            source,
            body: String::new(),
        })?;
        self.content_type = Some(content_type);
        self.body = Some(bytes);
        Ok(self)
    }
}

pub(crate) struct Response {
    pub status: u16,
    pub url: Url,
    pub body: Vec<u8>,
}

impl Response {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn ensure_success(self) -> Result<Self> {
        if self.is_success() {
            Ok(self)
        } else {
            Err(ApiError::from_response(self.status, &self.body).into())
        }
    }

    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(&self.body).map_err(|source| Error::Decode {
            source,
            body: truncate(&String::from_utf8_lossy(&self.body)),
        })
    }
}

pub(crate) struct Context<'a> {
    pub user_agent: &'a str,
}

pub(crate) trait Operation {
    type Output;

    fn request(&self, cx: &Context<'_>) -> Result<Request>;

    fn parse(&self, response: Response) -> Result<Self::Output>;
}

pub(crate) struct Transport {
    api_key: ApiKey,
    core: Url,
    iam: Url,
    user_agent: String,
}

impl Transport {
    pub fn new(api_key: ApiKey, core: Url, iam: Url, user_agent: String) -> Self {
        Self {
            api_key,
            core,
            iam,
            user_agent,
        }
    }

    pub fn context(&self) -> Context<'_> {
        Context {
            user_agent: &self.user_agent,
        }
    }

    pub fn url(&self, request: &Request) -> Result<Url> {
        let mut url = match request.host {
            Host::Core => self.core.clone(),
            Host::Iam => self.iam.clone(),
        };
        url.path_segments_mut()
            .expect("base urls are validated by the builder")
            .pop_if_empty()
            .extend(&request.segments);
        if !request.query.is_empty() {
            url.query_pairs_mut().extend_pairs(&request.query);
        }
        Ok(url)
    }

    pub fn headers(&self, request: &Request) -> Result<HeaderMap> {
        let invalid =
            |what| ValidationError::new("header", format!("{what} is not a valid header value"));
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, self.api_key.bearer());
        headers.insert(
            ACCEPT,
            HeaderValue::from_str(&request.accept).map_err(|_| invalid("accept"))?,
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&self.user_agent).map_err(|_| invalid("user agent"))?,
        );
        if let Some(content_type) = request.content_type {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
        }
        Ok(headers)
    }
}

pub(crate) fn validate_base_url(
    field: &'static str,
    url: &Url,
) -> std::result::Result<(), ValidationError> {
    if url.cannot_be_a_base() {
        return Err(ValidationError::new(
            field,
            "must be an absolute http(s) url",
        ));
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ValidationError::new(
            field,
            "must not contain credentials, a query or a fragment",
        ));
    }
    match url.scheme() {
        "https" => Ok(()),
        "http" if is_loopback(url) => Ok(()),
        _ => Err(ValidationError::new(
            field,
            "must use https (http is only allowed for loopback hosts)",
        )),
    }
}

fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

pub(crate) fn user_agent_token(
    field: &'static str,
    value: &str,
) -> std::result::Result<(), ValidationError> {
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_graphic()) || value.contains('/') {
        return Err(ValidationError::new(
            field,
            "must be non-empty printable ASCII without spaces or `/`",
        ));
    }
    Ok(())
}

pub(crate) fn compose_user_agent(
    custom: &[(String, String)],
) -> std::result::Result<String, ValidationError> {
    let mut parts: Vec<String> = custom
        .iter()
        .map(|(name, version)| format!("{name}/{version}"))
        .collect();
    parts.push(concat!("myparcel-rs/", env!("CARGO_PKG_VERSION")).to_owned());
    let joined = parts.join(" ");
    if joined.len() > MAX_USER_AGENT_LEN {
        return Err(ValidationError::new(
            "user agent",
            format!("is longer than {MAX_USER_AGENT_LEN} characters"),
        ));
    }
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn base_urls_must_be_https_unless_loopback() {
        assert!(validate_base_url("u", &url("https://api.myparcel.nl/")).is_ok());
        assert!(validate_base_url("u", &url("http://127.0.0.1:8080")).is_ok());
        assert!(validate_base_url("u", &url("http://localhost/")).is_ok());
        assert!(validate_base_url("u", &url("http://api.myparcel.nl/")).is_err());
        assert!(validate_base_url("u", &url("https://user:pw@api.myparcel.nl/")).is_err());
        assert!(validate_base_url("u", &url("https://api.myparcel.nl/?a=b")).is_err());
    }

    #[test]
    fn user_agent_lists_custom_parts_first() {
        let agent = compose_user_agent(&[("shop".into(), "1.2".into())]).unwrap();
        assert_eq!(
            agent,
            format!("shop/1.2 myparcel-rs/{}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn path_segments_are_encoded() {
        let transport = Transport::new(
            ApiKey::new("k").unwrap(),
            url("https://example.test/base/"),
            url("https://iam.test/"),
            String::new(),
        );
        let request = Request::new(
            Method::GET,
            Host::Core,
            vec!["a/b".into(), "1;2".into()],
            "x",
        )
        .query("q", "a b");
        assert_eq!(
            transport.url(&request).unwrap().as_str(),
            "https://example.test/base/a%2Fb/1;2?q=a+b"
        );
    }
}
