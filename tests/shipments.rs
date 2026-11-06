mod common;

use std::collections::BTreeSet;

use common::schema;
use common::{AUTH, error, respond, setup};
use myparcel_rs::country::CountryCode;
use myparcel_rs::enums::{Carrier, PackageType, ShipmentStatus, ShipmentType, TransactionStatus};
use myparcel_rs::labels::{A4Positions, LabelFormat};
use myparcel_rs::shipments::{
    Barcode, City, NewShipment, Order, Page, PageSize, Person, PostalCode, Recipient, RegionFilter,
    ShipmentBatch, ShipmentOption, ShipmentOptions, ShipmentQuery, Sort, Street,
};
use myparcel_rs::types::{NonEmpty, ReferenceIdentifier, ShipmentId, ShipmentIds, ShopId};
use myparcel_rs::{Error, Reported};
use serde_json::Value;
use time::macros::date;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, Request};

fn shipment(reference: Option<&str>) -> NewShipment {
    let shipment = NewShipment::new(
        Carrier::PostNl,
        Recipient::new(
            CountryCode::NL,
            PostalCode::new("2132 JE").unwrap(),
            City::new("Hoofddorp").unwrap(),
            Street::new("Antareslaan").unwrap(),
            Person::new("John Doe").unwrap(),
        ),
        ShipmentOptions::new(PackageType::Package).flag(ShipmentOption::Signature, true),
    );
    match reference {
        Some(reference) => {
            shipment.reference_identifier(ReferenceIdentifier::new(reference).unwrap())
        }
        None => shipment,
    }
}

#[tokio::test]
async fn create_sends_the_documented_request_and_matches_references() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/shipments"))
        .and(header("authorization", AUTH))
        .and(header(
            "content-type",
            "application/vnd.shipment+json;version=1.1",
        ))
        .and(query_param("format", "A4"))
        .and(query_param("positions", "2;3;4"))
        .respond_with(respond("shipments_created"))
        .expect(1)
        .mount(&server)
        .await;

    let batch = ShipmentBatch::new(vec![shipment(Some("order-1")), shipment(None)]).unwrap();
    let format = LabelFormat::A4(A4Positions::StartingAt(
        myparcel_rs::enums::LabelQuadrant::TopRightQuadrant,
    ));
    let created = client
        .shipments()
        .create(&batch, Some(&format))
        .await
        .unwrap();

    assert_eq!(created.shipments[0].id, ShipmentId(1001));
    assert_eq!(created.shipments[0].reference.as_deref(), Some("order-1"));
    let generated = created.shipments[1].reference.as_deref().unwrap();
    assert!(generated.starts_with("sdk_"), "{generated}");

    let requests = server.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(
        body["data"]["user_agent"]
            .as_str()
            .unwrap()
            .contains("myparcel-rs/")
    );
    assert_eq!(body["data"]["shipments"][0]["options"]["signature"], 1);
    assert_eq!(
        body["data"]["shipments"][1]["reference_identifier"],
        generated
    );
    let accept = requests[0].headers.get("accept").unwrap().to_str().unwrap();
    assert!(accept.starts_with("application/json;charset=utf-8;version=1.2"));
}

#[tokio::test]
async fn find_many_joins_ids_and_keeps_unknown_values() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/shipments/1001;1002"))
        .respond_with(respond("shipments"))
        .mount(&server)
        .await;

    let ids = ShipmentIds::new(vec![ShipmentId(1001), ShipmentId(1002)]).unwrap();
    let found = client.shipments().find_many(&ids).await.unwrap();
    assert_eq!(
        found[0].status,
        Some(Reported::Known(ShipmentStatus::PendingConcept))
    );
    assert_eq!(found[0].barcode.as_deref(), Some("3SABCD0123456"));
}

#[tokio::test]
async fn query_defaults_to_a_size_of_300_but_reference_search_sends_none() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/shipments"))
        .respond_with(respond("shipments"))
        .mount(&server)
        .await;

    client
        .shipments()
        .query(ShipmentQuery::new())
        .await
        .unwrap();
    client
        .shipments()
        .find_by_reference(&ReferenceIdentifier::new("order-*").unwrap())
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let query_of = |request: &Request| request.url.query().unwrap_or_default().to_owned();
    assert_eq!(query_of(&requests[0]), "size=300");
    assert_eq!(query_of(&requests[1]), "reference_identifier=order-*");
}

#[tokio::test]
async fn delete_accepts_no_content() {
    let (server, client) = setup().await;
    Mock::given(method("DELETE"))
        .and(path("/shipments/7"))
        .respond_with(wiremock::ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    client
        .shipments()
        .delete(&ShipmentIds::one(ShipmentId(7)))
        .await
        .unwrap();
}

#[tokio::test]
async fn api_errors_carry_the_status_and_message() {
    let (server, client) = setup().await;
    Mock::given(method("DELETE"))
        .respond_with(error(
            422,
            r#"{"message":"Invalid shipments","request_id":"r1","status_code":422,"errors":[{"code":3212,"message":"Shipment 7 cannot be deleted"}]}"#,
        ))
        .mount(&server)
        .await;

    let err = client
        .shipments()
        .delete(&ShipmentIds::one(ShipmentId(7)))
        .await
        .unwrap_err();
    let api = err.as_api().expect("api error");
    assert_eq!(api.status, 422);
    assert_eq!(api.request_id.as_deref(), Some("r1"));
    assert_eq!(api.details[0].code.as_deref(), Some("3212"));
    assert!(matches!(err, Error::Api(_)));
}

#[tokio::test]
async fn unauthorized_is_recognised() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .respond_with(error(401, r#"{"message":"Access Denied"}"#))
        .mount(&server)
        .await;
    let err = client
        .shipments()
        .query(ShipmentQuery::new())
        .await
        .unwrap_err();
    assert!(err.is_unauthorized());
}

#[tokio::test]
async fn every_query_filter_is_a_parameter_of_the_specification() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/shipments"))
        .respond_with(respond("shipments"))
        .mount(&server)
        .await;

    let query = ShipmentQuery::new()
        .barcode(&Barcode::new("3SABCD0123456").unwrap())
        .carrier(&NonEmpty::new(vec![Carrier::PostNl, Carrier::Dpd]).unwrap())
        .created(date!(2026 - 01 - 02))
        .from(date!(2026 - 01 - 03))
        .to(date!(2026 - 01 - 04))
        .delayed(true)
        .delivered(false)
        .dropoff_today(true)
        .external_identifier("ext-1")
        .filter_hidden_shops(true)
        .hidden(false)
        .include_secondary_shipments(true)
        .link_consumer_portal(true)
        .order(Order::Descending)
        .package_type(&NonEmpty::new(vec![PackageType::Package, PackageType::Mailbox]).unwrap())
        .page(Page::new(2).unwrap())
        .search("john")
        .reference_identifier(&ReferenceIdentifier::new("order-*").unwrap())
        .region(RegionFilter::Eu)
        .shipment_type(&NonEmpty::new(vec![ShipmentType::Standard, ShipmentType::Return]).unwrap())
        .shop_id(&NonEmpty::new(vec![ShopId(1), ShopId(2)]).unwrap())
        .size(PageSize::new(50).unwrap())
        .sort(Sort::Created)
        .status(
            &NonEmpty::new(vec![
                ShipmentStatus::PendingConcept,
                ShipmentStatus::Delivered,
            ])
            .unwrap(),
        )
        .transaction_status(TransactionStatus::Paid);
    client.shipments().query(query).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let sent: BTreeSet<String> = requests[0]
        .url
        .query_pairs()
        .map(|(name, _)| name.into_owned())
        .collect();

    let spec = schema::core();
    let parameters = &spec["paths"]["/shipments"]["get"]["parameters"];
    let known: BTreeSet<String> = parameters
        .as_array()
        .unwrap()
        .iter()
        .map(|parameter| match parameter.get("$ref") {
            Some(reference) => {
                let key = reference.as_str().unwrap().rsplit('/').next().unwrap();
                spec["components"]["parameters"][key]["name"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            }
            None => parameter["name"].as_str().unwrap().to_owned(),
        })
        .collect();

    assert_eq!(sent.len(), 25);
    let unknown: Vec<_> = sent.difference(&known).collect();
    assert!(unknown.is_empty(), "not in the specification: {unknown:?}");
    let missing: Vec<_> = known.difference(&sent).collect();
    assert_eq!(
        missing,
        vec!["id"],
        "filters of the specification that cannot be set"
    );
}
