#![cfg(feature = "blocking")]

mod common;

use myparcel_rs::ApiKey;
use myparcel_rs::blocking::Client;
use myparcel_rs::types::{ShipmentId, ShipmentIds};
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer};

#[test]
fn the_blocking_client_runs_the_same_operations() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let server = runtime.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/shipments/1001"))
            .and(header("authorization", common::AUTH))
            .respond_with(common::respond("shipments"))
            .mount(&server)
            .await;
        server
    });

    let client = Client::builder(ApiKey::new("test-key").unwrap())
        .base_url(Url::parse(&server.uri()).unwrap())
        .build_blocking()
        .unwrap();
    let found = client.shipments().find(ShipmentId(1001)).unwrap().unwrap();
    assert_eq!(found.id, ShipmentId(1001));
    let _ = ShipmentIds::one(ShipmentId(1));
}

#[test]
fn blocking_labels_and_account_checks_behave_like_the_async_ones() {
    use myparcel_rs::labels::LabelFormat;

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let server = runtime.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/shipment_labels/7"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_raw(b"%PDF-1.4 x".to_vec(), "application/pdf"),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/whoami"))
            .respond_with(
                wiremock::ResponseTemplate::new(401)
                    .set_body_raw(r#"{"message":"no"}"#, "application/json"),
            )
            .mount(&server)
            .await;
        server
    });
    let client = Client::builder(ApiKey::new("test-key").unwrap())
        .base_url(Url::parse(&server.uri()).unwrap())
        .iam_base_url(Url::parse(&server.uri()).unwrap())
        .build_blocking()
        .unwrap();

    let pdf = client
        .labels()
        .pdf(&ShipmentIds::one(ShipmentId(7)), &LabelFormat::A6)
        .unwrap();
    assert!(pdf.bytes.starts_with(b"%PDF-1.4"));
    assert!(!client.account().is_valid().unwrap());
}
