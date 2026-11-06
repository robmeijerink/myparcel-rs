#![allow(dead_code)]

pub mod schema;

use myparcel_rs::{ApiKey, Client};
use url::Url;
use wiremock::{MockServer, ResponseTemplate};

pub const AUTH: &str = "bearer dGVzdC1rZXk=";

pub async fn setup() -> (MockServer, Client) {
    let server = MockServer::start().await;
    let client = Client::builder(ApiKey::new("test-key").unwrap())
        .base_url(Url::parse(&server.uri()).unwrap())
        .iam_base_url(Url::parse(&format!("{}/iam/", server.uri())).unwrap())
        .build()
        .unwrap();
    (server, client)
}

pub fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

pub fn respond(name: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(fixture(name), "application/json")
}

pub fn error(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_raw(body.to_owned(), "application/json")
}

pub fn token_json(thumbprint: &str, suffix: &str) -> serde_json::Value {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let payload =
        URL_SAFE_NO_PAD.encode(serde_json::json!({"cnf": {"jkt": thumbprint}}).to_string());
    serde_json::json!({
        "accessToken": format!("header.{payload}.{suffix}"),
        "connectionId": "conn-1",
        "tokenType": "DPoP",
        "expiresIn": 300,
        "refreshExpiresIn": 3600,
        "scope": "integration"
    })
}
