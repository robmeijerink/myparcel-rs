#![cfg(feature = "connect")]

use std::sync::Mutex;

mod common;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use common::{schema, token_json};
use myparcel_rs::connect::{
    CallbackQuery, ConnectConfig, ConnectError, ConnectPlatform, ConnectService, ConnectStorage,
    EncryptionKey, Installation, Nonces, Order, OrderResult, OrderStatus, ShopName, ShopUrl,
    SourceId, StorageError, Token,
};
use myparcel_rs::types::NonEmpty;
use serde_json::{Value, json};
use time::macros::datetime;
use url::Url;
use wiremock::matchers::{header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Default)]
struct Memory {
    state: Mutex<(Option<Installation>, Option<Token>, Option<Nonces>)>,
}

struct Shared(std::sync::Arc<Memory>);

impl ConnectStorage for Shared {
    async fn load_installation(&self) -> Result<Option<Installation>, StorageError> {
        self.0.load_installation().await
    }

    async fn save_installation(&self, installation: &Installation) -> Result<(), StorageError> {
        self.0.save_installation(installation).await
    }

    async fn load_token(&self) -> Result<Option<Token>, StorageError> {
        self.0.load_token().await
    }

    async fn save_token(&self, token: Option<&Token>) -> Result<(), StorageError> {
        self.0.save_token(token).await
    }

    async fn load_nonces(&self) -> Result<Option<Nonces>, StorageError> {
        self.0.load_nonces().await
    }

    async fn save_nonces(&self, nonces: Option<&Nonces>) -> Result<(), StorageError> {
        self.0.save_nonces(nonces).await
    }

    async fn clear(&self) -> Result<(), StorageError> {
        self.0.clear().await
    }
}

impl ConnectStorage for Memory {
    async fn load_installation(&self) -> Result<Option<Installation>, StorageError> {
        Ok(self.state.lock().unwrap().0.clone())
    }

    async fn save_installation(&self, installation: &Installation) -> Result<(), StorageError> {
        self.state.lock().unwrap().0 = Some(installation.clone());
        Ok(())
    }

    async fn load_token(&self) -> Result<Option<Token>, StorageError> {
        Ok(self.state.lock().unwrap().1.clone())
    }

    async fn save_token(&self, token: Option<&Token>) -> Result<(), StorageError> {
        self.state.lock().unwrap().1 = token.cloned();
        Ok(())
    }

    async fn load_nonces(&self) -> Result<Option<Nonces>, StorageError> {
        Ok(self.state.lock().unwrap().2.clone())
    }

    async fn save_nonces(&self, nonces: Option<&Nonces>) -> Result<(), StorageError> {
        self.state.lock().unwrap().2 = nonces.cloned();
        Ok(())
    }

    async fn clear(&self) -> Result<(), StorageError> {
        *self.state.lock().unwrap() = Default::default();
        Ok(())
    }
}

const HTU: &str = "https://shopify.ecommerce.api.myparcel.nl/connect/token";

fn service(server: &MockServer, secret: &str) -> ConnectService<Memory> {
    let config = ConnectConfig::new(
        ConnectPlatform::Shopify,
        EncryptionKey::new(secret).unwrap(),
    )
    .with_base_url(Url::parse(&server.uri()).unwrap())
    .unwrap();
    ConnectService::new(config, Memory::default()).unwrap()
}

fn query_of(url: &Url) -> std::collections::HashMap<String, String> {
    url.query_pairs().into_owned().collect()
}

fn claims_of(proof: &str) -> Value {
    let claims = proof.split('.').nth(1).unwrap();
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims).unwrap()).unwrap()
}

fn callback(nonce: &str) -> CallbackQuery {
    CallbackQuery::from_pairs([
        ("nonce", nonce.to_owned()),
        ("code", "C".repeat(43)),
        ("htm", "POST".to_owned()),
        ("htu", HTU.to_owned()),
    ])
}

async fn start(service: &ConnectService<Memory>) -> (String, String) {
    start_with(service).await
}

async fn start_with<S: ConnectStorage>(service: &ConnectService<S>) -> (String, String) {
    let url = service
        .start(
            &ShopName::new("Test shop").unwrap(),
            &"https://shop.example".parse::<ShopUrl>().unwrap(),
            None,
        )
        .await
        .unwrap();
    let query = query_of(&url);
    (query["nonce"].clone(), query["jkt"].clone())
}

#[tokio::test]
async fn connecting_retries_with_the_requested_nonce_and_pushes_orders() {
    let server = MockServer::start().await;
    let service = service(&server, "secret");
    let (nonce, thumbprint) = start(&service).await;

    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(
            ResponseTemplate::new(400)
                .insert_header("DPoP-Nonce", "server-nonce")
                .set_body_json(json!({"error": "use_dpop_nonce"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .and(header_exists("dpop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_json(&thumbprint, "one")))
        .mount(&server)
        .await;

    assert!(!service.is_connected().await);
    service.handle_callback(&callback(&nonce)).await.unwrap();
    assert!(service.is_connected().await);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    let first = claims_of(requests[0].headers.get("dpop").unwrap().to_str().unwrap());
    let second = claims_of(requests[1].headers.get("dpop").unwrap().to_str().unwrap());
    assert!(first.get("nonce").is_none());
    assert_eq!(second["nonce"], "server-nonce");
    assert_eq!(second["htm"], "POST");
    assert_eq!(second["htu"], HTU);
    assert_eq!(
        requests[1].body,
        br#"{"code":"CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC"}"#
    );

    Mock::given(method("POST"))
        .and(path("/connect/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_json(&thumbprint, "two")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/webhook/orders"))
        .respond_with(ResponseTemplate::new(401))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/webhook/orders"))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!([
            {"status": 202},
            {"type": "urn:problem", "title": "Bad order", "status": 400, "detail": "no lines"}
        ])))
        .mount(&server)
        .await;

    let order = |id: &str| {
        Order::new(
            datetime!(2026-01-02 03:04:05 UTC),
            SourceId::new(id).unwrap(),
            OrderStatus::Open,
        )
    };
    let batch = NonEmpty::new(vec![order("A"), order("B")]).unwrap();
    let results = service.push_orders(&batch).await.unwrap();
    assert_eq!(results[0], OrderResult::Accepted);
    assert!(
        matches!(&results[1], OrderResult::Problem(problem) if problem.title.as_deref() == Some("Bad order"))
    );

    let requests = server.received_requests().await.unwrap();
    let pushes: Vec<_> = requests
        .iter()
        .filter(|r| r.url.path() == "/webhook/orders")
        .collect();
    assert_eq!(pushes.len(), 2);
    let authorization = pushes[1]
        .headers
        .get("authorization")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(authorization.starts_with("DPoP header."), "{authorization}");
    assert!(
        authorization.ends_with(".two"),
        "the retry uses the refreshed token"
    );
    let proof = claims_of(pushes[1].headers.get("dpop").unwrap().to_str().unwrap());
    assert_eq!(proof["htm"], "POST");
    assert!(proof.get("ath").is_some());
    let sent: Value = serde_json::from_slice(&pushes[1].body).unwrap();
    assert_eq!(sent[0]["sourceId"], "A");
}

#[tokio::test]
async fn a_callback_needs_the_nonce_of_the_start_and_uses_it_once() {
    let server = MockServer::start().await;
    let service = service(&server, "secret");
    let (nonce, thumbprint) = start(&service).await;

    let wrong = service
        .handle_callback(&callback("not-the-nonce"))
        .await
        .unwrap_err();
    assert!(matches!(wrong, ConnectError::InvalidCallback(_)));

    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_json(&thumbprint, "one")))
        .mount(&server)
        .await;
    service.handle_callback(&callback(&nonce)).await.unwrap();
    let replay = service
        .handle_callback(&callback(&nonce))
        .await
        .unwrap_err();
    assert!(matches!(replay, ConnectError::InvalidCallback(_)));
}

#[tokio::test]
async fn a_token_bound_to_another_key_is_refused() {
    let server = MockServer::start().await;
    let service = service(&server, "secret");
    let (nonce, _) = start(&service).await;
    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(token_json("someone-elses-key", "x")),
        )
        .mount(&server)
        .await;
    let error = service
        .handle_callback(&callback(&nonce))
        .await
        .unwrap_err();
    assert!(matches!(error, ConnectError::KeyMismatch));
    assert!(!service.is_connected().await);
}

#[tokio::test]
async fn callback_errors_are_translated() {
    let server = MockServer::start().await;
    let service = service(&server, "secret");
    let (nonce, _) = start(&service).await;
    let query =
        CallbackQuery::from_pairs([("nonce", nonce), ("error", "access_denied".to_owned())]);
    assert!(matches!(
        service.handle_callback(&query).await.unwrap_err(),
        ConnectError::AccessDenied
    ));
}

#[tokio::test]
async fn a_revoked_refresh_token_clears_the_connection() {
    let server = MockServer::start().await;
    let service = service(&server, "secret");
    let (nonce, thumbprint) = start(&service).await;
    let mut body = token_json(&thumbprint, "one");
    body["expiresIn"] = json!(1);
    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/connect/refresh"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": "invalid_grant"})))
        .mount(&server)
        .await;
    service.handle_callback(&callback(&nonce)).await.unwrap();

    let error = service.access_token().await.unwrap_err();
    assert!(matches!(error, ConnectError::ConnectionRevoked));
    assert!(!service.is_connected().await);
}

#[tokio::test]
async fn a_wrong_encryption_key_is_reported_as_such() {
    let server = MockServer::start().await;
    let storage = std::sync::Arc::new(Memory::default());
    let config = ConnectConfig::new(
        ConnectPlatform::Shopify,
        EncryptionKey::new("secret").unwrap(),
    )
    .with_base_url(Url::parse(&server.uri()).unwrap())
    .unwrap();
    let first = ConnectService::new(config, Shared(storage.clone())).unwrap();
    start_with(&first).await;

    let config = ConnectConfig::new(
        ConnectPlatform::Shopify,
        EncryptionKey::new("other").unwrap(),
    )
    .with_base_url(Url::parse(&server.uri()).unwrap())
    .unwrap();
    let second = ConnectService::new(config, Shared(storage)).unwrap();
    let error = second
        .start(
            &ShopName::new("Test shop").unwrap(),
            &"https://shop.example".parse::<ShopUrl>().unwrap(),
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ConnectError::Decryption));
}

#[tokio::test]
async fn uninstalling_removes_everything() {
    let server = MockServer::start().await;
    let service = service(&server, "secret");
    start(&service).await;
    service.uninstall().await.unwrap();
    let error = service.handle_callback(&callback("x")).await.unwrap_err();
    assert!(matches!(error, ConnectError::NotConnected));
}

#[tokio::test]
async fn the_start_url_carries_a_config_that_satisfies_the_schema() {
    let server = MockServer::start().await;
    let service = service(&server, "secret");
    let url = service
        .start(
            &ShopName::new("Test shop").unwrap(),
            &"https://shop.example/sub".parse::<ShopUrl>().unwrap(),
            Some(&myparcel_rs::connect::PluginVersion::new("1.2.3").unwrap()),
        )
        .await
        .unwrap();
    let query = query_of(&url);
    let config: Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(&query["config"]).unwrap()).unwrap();
    assert_eq!(config["shopUrl"], "https://shop.example/sub");
    schema::assert_valid(&schema::connect(), "ConnectStartConfig", &config);
    assert!(query["nonce"].len() >= 16 && query["nonce"].len() <= 256);
    assert!(!query["jkt"].is_empty());
    assert!(query["config"].len() <= 1024);
}

fn shared_service(
    server: &MockServer,
    storage: &std::sync::Arc<Memory>,
    leeway: u64,
) -> ConnectService<Shared> {
    let config = ConnectConfig::new(
        ConnectPlatform::Shopify,
        EncryptionKey::new("secret").unwrap(),
    )
    .with_base_url(Url::parse(&server.uri()).unwrap())
    .unwrap()
    .with_expiry_leeway(std::time::Duration::from_secs(leeway));
    ConnectService::new(config, Shared(storage.clone())).unwrap()
}

fn thumbprint_of(service_url: &Url) -> String {
    query_of(service_url)["jkt"].clone()
}

#[tokio::test]
async fn an_expired_start_nonce_is_refused() {
    let server = MockServer::start().await;
    let storage = std::sync::Arc::new(Memory::default());
    let service = shared_service(&server, &storage, 30);
    let url = service
        .start(
            &ShopName::new("Shop").unwrap(),
            &"https://shop.example".parse::<ShopUrl>().unwrap(),
            None,
        )
        .await
        .unwrap();
    let nonce = query_of(&url)["nonce"].clone();
    {
        let mut state = storage.state.lock().unwrap();
        state.2.as_mut().unwrap().start_nonce_expires_at = Some(0);
    }
    let error = service
        .handle_callback(&callback(&nonce))
        .await
        .unwrap_err();
    assert!(matches!(error, ConnectError::InvalidCallback(_)));
}

#[tokio::test]
async fn a_server_that_keeps_asking_for_a_nonce_is_given_up_on() {
    let server = MockServer::start().await;
    let storage = std::sync::Arc::new(Memory::default());
    let service = shared_service(&server, &storage, 30);
    let url = service
        .start(
            &ShopName::new("Shop").unwrap(),
            &"https://shop.example".parse::<ShopUrl>().unwrap(),
            None,
        )
        .await
        .unwrap();
    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(
            ResponseTemplate::new(400)
                .insert_header("DPoP-Nonce", "again")
                .set_body_json(json!({"error": "use_dpop_nonce"})),
        )
        .mount(&server)
        .await;
    let error = service
        .handle_callback(&callback(&query_of(&url)["nonce"]))
        .await
        .unwrap_err();
    assert!(matches!(error, ConnectError::NonceRequired));
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn asking_for_a_nonce_without_providing_one_is_not_retried() {
    let server = MockServer::start().await;
    let storage = std::sync::Arc::new(Memory::default());
    let service = shared_service(&server, &storage, 30);
    let url = service
        .start(
            &ShopName::new("Shop").unwrap(),
            &"https://shop.example".parse::<ShopUrl>().unwrap(),
            None,
        )
        .await
        .unwrap();
    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": "use_dpop_nonce"})))
        .mount(&server)
        .await;
    let error = service
        .handle_callback(&callback(&query_of(&url)["nonce"]))
        .await
        .unwrap_err();
    assert!(matches!(error, ConnectError::NonceRequired));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_token_that_expires_within_the_leeway_is_refreshed_first() {
    let server = MockServer::start().await;
    let storage = std::sync::Arc::new(Memory::default());
    let service = shared_service(&server, &storage, 200);
    let url = service
        .start(
            &ShopName::new("Shop").unwrap(),
            &"https://shop.example".parse::<ShopUrl>().unwrap(),
            None,
        )
        .await
        .unwrap();
    let thumbprint = thumbprint_of(&url);
    let mut first = token_json(&thumbprint, "one");
    first["expiresIn"] = json!(100);
    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(first))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/connect/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_json(&thumbprint, "two")))
        .expect(1)
        .mount(&server)
        .await;
    service
        .handle_callback(&callback(&query_of(&url)["nonce"]))
        .await
        .unwrap();

    let token = service.access_token().await.unwrap();
    assert!(token.expose().ends_with(".two"));
    assert!(!format!("{token:?}").contains("two"));
    assert!(
        service
            .access_token()
            .await
            .unwrap()
            .expose()
            .ends_with(".two")
    );
}

#[tokio::test]
async fn a_token_that_is_still_valid_is_not_refreshed() {
    let server = MockServer::start().await;
    let storage = std::sync::Arc::new(Memory::default());
    let service = shared_service(&server, &storage, 30);
    let url = service
        .start(
            &ShopName::new("Shop").unwrap(),
            &"https://shop.example".parse::<ShopUrl>().unwrap(),
            None,
        )
        .await
        .unwrap();
    let thumbprint = thumbprint_of(&url);
    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_json(&thumbprint, "one")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/connect/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_json(&thumbprint, "two")))
        .expect(0)
        .mount(&server)
        .await;
    service
        .handle_callback(&callback(&query_of(&url)["nonce"]))
        .await
        .unwrap();
    assert!(
        service
            .access_token()
            .await
            .unwrap()
            .expose()
            .ends_with(".one")
    );
}

#[tokio::test]
async fn disconnecting_keeps_the_key_so_the_shop_can_connect_again() {
    let server = MockServer::start().await;
    let storage = std::sync::Arc::new(Memory::default());
    let service = shared_service(&server, &storage, 30);
    let start = || async {
        service
            .start(
                &ShopName::new("Shop").unwrap(),
                &"https://shop.example".parse::<ShopUrl>().unwrap(),
                None,
            )
            .await
            .unwrap()
    };
    let first = start().await;
    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(token_json(&thumbprint_of(&first), "one")),
        )
        .mount(&server)
        .await;
    service
        .handle_callback(&callback(&query_of(&first)["nonce"]))
        .await
        .unwrap();
    assert!(service.is_connected().await);

    service.disconnect().await.unwrap();
    assert!(!service.is_connected().await);
    assert!(matches!(
        service.access_token().await.unwrap_err(),
        ConnectError::NotConnected
    ));
    assert_eq!(thumbprint_of(&start().await), thumbprint_of(&first));
}

#[tokio::test]
async fn a_malformed_code_never_reaches_the_network() {
    let server = MockServer::start().await;
    let service = service(&server, "secret");
    let (nonce, _) = start(&service).await;
    let query = CallbackQuery::from_pairs([
        ("nonce", nonce),
        ("code", "too-short".to_owned()),
        ("htm", "POST".to_owned()),
        ("htu", HTU.to_owned()),
    ]);
    assert!(matches!(
        service.handle_callback(&query).await.unwrap_err(),
        ConnectError::InvalidCallback(_)
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn an_unknown_sales_channel_is_reported() {
    let server = MockServer::start().await;
    let service = service(&server, "secret");
    let (nonce, _) = start(&service).await;
    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({"error": "invalid_sales_channel"})),
        )
        .mount(&server)
        .await;
    let error = service
        .handle_callback(&callback(&nonce))
        .await
        .unwrap_err();
    assert!(matches!(error, ConnectError::InvalidSalesChannel));
}

#[tokio::test]
async fn a_rejected_push_is_not_retried_more_than_once() {
    let server = MockServer::start().await;
    let storage = std::sync::Arc::new(Memory::default());
    let service = shared_service(&server, &storage, 30);
    let url = service
        .start(
            &ShopName::new("Shop").unwrap(),
            &"https://shop.example".parse::<ShopUrl>().unwrap(),
            None,
        )
        .await
        .unwrap();
    let thumbprint = thumbprint_of(&url);
    connect_with(&service, &server, &url, token_json(&thumbprint, "one")).await;
    Mock::given(method("POST"))
        .and(path("/connect/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_json(&thumbprint, "two")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/webhook/orders"))
        .respond_with(ResponseTemplate::new(401))
        .expect(2)
        .mount(&server)
        .await;

    let order = Order::new(
        datetime!(2026-01-02 03:04:05 UTC),
        SourceId::new("A").unwrap(),
        OrderStatus::Open,
    );
    let error = service
        .push_orders(&NonEmpty::one(order))
        .await
        .unwrap_err();
    assert!(matches!(error, ConnectError::Http { status: 401, .. }));
}

async fn connect_with(
    service: &ConnectService<Shared>,
    server: &MockServer,
    url: &Url,
    body: Value,
) {
    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
    service
        .handle_callback(&callback(&query_of(url)["nonce"]))
        .await
        .unwrap();
}

struct Failing;

impl ConnectStorage for Failing {
    async fn load_installation(&self) -> Result<Option<Installation>, StorageError> {
        Err("disk full".into())
    }

    async fn save_installation(&self, _: &Installation) -> Result<(), StorageError> {
        Err("disk full".into())
    }

    async fn load_token(&self) -> Result<Option<Token>, StorageError> {
        Err("disk full".into())
    }

    async fn save_token(&self, _: Option<&Token>) -> Result<(), StorageError> {
        Err("disk full".into())
    }

    async fn load_nonces(&self) -> Result<Option<Nonces>, StorageError> {
        Err("disk full".into())
    }

    async fn save_nonces(&self, _: Option<&Nonces>) -> Result<(), StorageError> {
        Err("disk full".into())
    }

    async fn clear(&self) -> Result<(), StorageError> {
        Err("disk full".into())
    }
}

#[tokio::test]
async fn a_failing_storage_is_reported_and_nothing_is_sent() {
    let server = MockServer::start().await;
    let config = ConnectConfig::new(
        ConnectPlatform::Shopify,
        EncryptionKey::new("secret").unwrap(),
    )
    .with_base_url(Url::parse(&server.uri()).unwrap())
    .unwrap();
    let service = ConnectService::new(config, Failing).unwrap();
    let error = service
        .start(
            &ShopName::new("Shop").unwrap(),
            &"https://shop.example".parse::<ShopUrl>().unwrap(),
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(&error, ConnectError::Storage(inner) if inner.to_string() == "disk full"));
    assert!(!service.is_connected().await);
    assert!(matches!(
        service.uninstall().await.unwrap_err(),
        ConnectError::Storage(_)
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
