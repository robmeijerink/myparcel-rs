mod common;

use common::{respond, setup};
use myparcel_rs::country::CountryCode;
use myparcel_rs::enums::{Carrier, PackageType};
use myparcel_rs::shipments::{
    City, NewShipment, Person, PostalCode, Recipient, ShipmentBatch, ShipmentOptions,
    ShipmentQuery, Street,
};
use myparcel_rs::{ApiKey, Client, Error};
use serde_json::Value;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

fn key() -> ApiKey {
    ApiKey::new("super-secret-key").unwrap()
}

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

#[tokio::test]
async fn redirects_are_not_followed() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/shipments"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/elsewhere"))
        .mount(&server)
        .await;
    Mock::given(path("/elsewhere"))
        .respond_with(respond("shipments"))
        .expect(0)
        .mount(&server)
        .await;

    let error = client
        .shipments()
        .query(ShipmentQuery::new())
        .await
        .unwrap_err();
    assert_eq!(error.as_api().map(|api| api.status), Some(302));
}

#[test]
fn a_plain_http_base_url_is_refused_unless_it_is_loopback() {
    for refused in [
        "http://api.myparcel.nl/",
        "http://192.0.2.1/",
        "https://user:pw@api.myparcel.nl/",
        "https://api.myparcel.nl/?x=1",
    ] {
        let result = Client::builder(key()).base_url(url(refused)).build();
        assert!(matches!(result, Err(Error::Validation(_))), "{refused}");
        let result = Client::builder(key()).iam_base_url(url(refused)).build();
        assert!(matches!(result, Err(Error::Validation(_))), "{refused}");
    }
    for accepted in [
        "https://api.myparcel.nl/",
        "http://127.0.0.1:8080/",
        "http://localhost/",
        "http://[::1]/",
    ] {
        assert!(
            Client::builder(key())
                .base_url(url(accepted))
                .build()
                .is_ok(),
            "{accepted}"
        );
    }
}

#[test]
fn a_user_agent_that_could_break_the_header_is_refused() {
    for (platform, version) in [
        ("a/b", "1"),
        ("a b", "1"),
        ("a", "1\r\nX: y"),
        ("", "1"),
        ("a", ""),
        ("é", "1"),
    ] {
        let result = Client::builder(key()).user_agent(platform, version).build();
        assert!(
            matches!(result, Err(Error::Validation(_))),
            "{platform:?} {version:?}"
        );
    }
    let too_long = Client::builder(key())
        .user_agent("a".repeat(250), "1")
        .build();
    assert!(matches!(too_long, Err(Error::Validation(_))));
}

#[test]
fn the_key_is_not_shown_by_debug() {
    let builder = Client::builder(key());
    assert!(!format!("{builder:?}").contains("super-secret-key"));
    let client = Client::new(key()).unwrap();
    assert!(!format!("{client:?}").contains("super-secret-key"));
    assert!(!format!("{:?}", key()).contains("super-secret-key"));
}

#[tokio::test]
async fn the_user_agent_is_sent_in_the_header_and_in_the_body() {
    let (server, _) = setup().await;
    let client = Client::builder(key())
        .base_url(url(&server.uri()))
        .user_agent("WooCommerce", "9.1")
        .build()
        .unwrap();
    Mock::given(method("POST"))
        .respond_with(respond("shipments_created"))
        .mount(&server)
        .await;

    let recipient = Recipient::new(
        CountryCode::NL,
        PostalCode::new("2132 JE").unwrap(),
        City::new("Hoofddorp").unwrap(),
        Street::new("Antareslaan").unwrap(),
        Person::new("John Doe").unwrap(),
    );
    let shipment = NewShipment::new(
        Carrier::PostNl,
        recipient,
        ShipmentOptions::new(PackageType::Package),
    );
    client
        .shipments()
        .create(&ShipmentBatch::one(shipment), None)
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let header = requests[0]
        .headers
        .get("user-agent")
        .unwrap()
        .to_str()
        .unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        header,
        format!("WooCommerce/9.1 myparcel-rs/{}", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(body["data"]["user_agent"], header);
}

#[tokio::test]
async fn nothing_is_retried() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429))
        .expect(1)
        .mount(&server)
        .await;
    assert!(
        client
            .shipments()
            .query(ShipmentQuery::new())
            .await
            .is_err()
    );
}
