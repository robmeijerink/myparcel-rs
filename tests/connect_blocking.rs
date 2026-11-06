#![cfg(all(feature = "connect", feature = "blocking"))]

mod common;

use std::sync::Mutex;

use common::token_json;
use myparcel_rs::connect::{
    BlockingConnectService, BlockingConnectStorage, CallbackQuery, ConnectConfig, ConnectError,
    ConnectPlatform, EncryptionKey, Installation, Nonces, Order, OrderResult, OrderStatus,
    ShopName, ShopUrl, SourceId, StorageError, Token,
};
use myparcel_rs::types::NonEmpty;
use serde_json::json;
use time::macros::datetime;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Default)]
struct Memory {
    state: Mutex<(Option<Installation>, Option<Token>, Option<Nonces>)>,
}

impl BlockingConnectStorage for Memory {
    fn load_installation(&self) -> Result<Option<Installation>, StorageError> {
        Ok(self.state.lock().unwrap().0.clone())
    }

    fn save_installation(&self, installation: &Installation) -> Result<(), StorageError> {
        self.state.lock().unwrap().0 = Some(installation.clone());
        Ok(())
    }

    fn load_token(&self) -> Result<Option<Token>, StorageError> {
        Ok(self.state.lock().unwrap().1.clone())
    }

    fn save_token(&self, token: Option<&Token>) -> Result<(), StorageError> {
        self.state.lock().unwrap().1 = token.cloned();
        Ok(())
    }

    fn load_nonces(&self) -> Result<Option<Nonces>, StorageError> {
        Ok(self.state.lock().unwrap().2.clone())
    }

    fn save_nonces(&self, nonces: Option<&Nonces>) -> Result<(), StorageError> {
        self.state.lock().unwrap().2 = nonces.cloned();
        Ok(())
    }

    fn clear(&self) -> Result<(), StorageError> {
        *self.state.lock().unwrap() = Default::default();
        Ok(())
    }
}

#[test]
fn the_blocking_service_connects_and_pushes_orders() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let server = runtime.block_on(MockServer::start());

    let config = ConnectConfig::new(
        ConnectPlatform::Shopify,
        EncryptionKey::new("secret").unwrap(),
    )
    .with_base_url(Url::parse(&server.uri()).unwrap())
    .unwrap();
    let service = BlockingConnectService::new(config, Memory::default()).unwrap();

    assert!(!service.is_connected());
    let url = service
        .start(
            &ShopName::new("Shop").unwrap(),
            &"https://shop.example".parse::<ShopUrl>().unwrap(),
            None,
        )
        .unwrap();
    let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();

    runtime.block_on(async {
        Mock::given(method("POST"))
            .and(path("/connect/token"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(token_json(&query["jkt"], "one")),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/webhook/orders"))
            .respond_with(ResponseTemplate::new(202).set_body_json(json!([{"status": 202}])))
            .mount(&server)
            .await;
    });

    let callback = CallbackQuery::from_pairs([
        ("nonce", query["nonce"].clone()),
        ("code", "C".repeat(43)),
        ("htm", "POST".to_owned()),
        (
            "htu",
            "https://shopify.ecommerce.api.myparcel.nl/connect/token".to_owned(),
        ),
    ]);
    service.handle_callback(&callback).unwrap();
    assert!(service.is_connected());

    let order = Order::new(
        datetime!(2026-01-02 03:04:05 UTC),
        SourceId::new("A").unwrap(),
        OrderStatus::Open,
    );
    assert_eq!(
        service.push_orders(&NonEmpty::one(order)).unwrap(),
        vec![OrderResult::Accepted]
    );

    service.disconnect().unwrap();
    assert!(matches!(
        service.access_token().unwrap_err(),
        ConnectError::NotConnected
    ));
    service.uninstall().unwrap();
}
