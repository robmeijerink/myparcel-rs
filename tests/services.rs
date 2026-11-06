mod common;

use common::{AUTH, error, respond, setup};
use myparcel_rs::account::Feature;
use myparcel_rs::capabilities::{CapabilitiesRequest, CapabilityOption, ContractCarrier};
use myparcel_rs::country::CountryCode;
use myparcel_rs::enums::{Carrier, CarrierV2, ShipmentStatus};
use myparcel_rs::labels::LabelFormat;
use myparcel_rs::returns::{RelatedReturn, ReturnCarrier};
use myparcel_rs::shipments::EmailAddress;
use myparcel_rs::types::{NonEmpty, ShipmentId, ShipmentIds, WebhookSubscriptionId};
use myparcel_rs::webhooks::{Hook, WebhookUrl};
use myparcel_rs::{Error, Reported};
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

fn ids() -> ShipmentIds {
    ShipmentIds::new(vec![ShipmentId(1), ShipmentId(2)]).unwrap()
}

#[tokio::test]
async fn pdf_labels_are_validated_and_named() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/shipment_labels/1;2"))
        .and(header("accept", "application/pdf"))
        .and(query_param("format", "A6"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(b"%PDF-1.7 data".to_vec(), "application/pdf"),
        )
        .mount(&server)
        .await;

    let pdf = client.labels().pdf(&ids(), &LabelFormat::A6).await.unwrap();
    assert!(pdf.bytes.starts_with(b"%PDF-1.7"));
    assert!(pdf.filename.starts_with("myparcel-label-") && pdf.filename.ends_with(".pdf"));
}

#[tokio::test]
async fn something_that_is_not_a_pdf_is_rejected() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(b"<html>".to_vec(), "text/html"))
        .mount(&server)
        .await;
    let err = client
        .labels()
        .pdf(&ids(), &LabelFormat::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::UnexpectedResponse(_)));
}

#[tokio::test]
async fn unpaid_labels_report_the_payment() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(402)
                .set_body_raw(common::fixture("payment_required"), "application/json"),
        )
        .mount(&server)
        .await;
    match client
        .labels()
        .pdf(&ids(), &LabelFormat::default())
        .await
        .unwrap_err()
    {
        Error::PaymentRequired(instructions) => {
            assert_eq!(
                instructions[0].payment_url.as_deref(),
                Some("https://pay.example/p1")
            );
            assert!(!instructions[0].paid);
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn labels_that_are_held_back_say_which_shipments_and_why() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(header("accept", "application/vnd.shipment_label_link+json"))
        .respond_with(
            ResponseTemplate::new(402)
                .set_body_raw(common::fixture("labels_held_back"), "application/json"),
        )
        .mount(&server)
        .await;
    match client
        .labels()
        .link(&ids(), &LabelFormat::default())
        .await
        .unwrap_err()
    {
        Error::LabelsHeldBack(held) => {
            assert_eq!(held.unpaid.len(), 1);
            assert_eq!(held.unpaid[0].shipment_id, ShipmentId(1));
            assert_eq!(held.unpaid[0].price_after_vat.amount, Some(605));
            assert_eq!(held.blocked[0].shipment_id, ShipmentId(2));
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn label_links_are_returned_as_given() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(header("accept", "application/vnd.shipment_label_link+json"))
        .respond_with(respond("label_link"))
        .mount(&server)
        .await;
    let url = client
        .labels()
        .link(&ids(), &LabelFormat::default())
        .await
        .unwrap();
    assert_eq!(url.as_str(), "https://api.myparcel.nl/pdfs/abc123");
}

#[tokio::test]
async fn a_relative_label_link_is_made_absolute() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"data":{"pdf":{"url":"/pdfs/abc123"}}}"#,
            "application/json",
        ))
        .mount(&server)
        .await;
    let url = client
        .labels()
        .link(&ids(), &LabelFormat::default())
        .await
        .unwrap();
    assert_eq!(url.as_str(), format!("{}/pdfs/abc123", server.uri()));
}

#[tokio::test]
async fn track_traces_are_keyed_by_shipment() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/tracktraces/1001"))
        .respond_with(respond("tracktraces"))
        .mount(&server)
        .await;
    let traces = client
        .track_traces()
        .fetch(&ShipmentIds::one(ShipmentId(1001)))
        .await
        .unwrap();
    let trace = &traces[&ShipmentId(1001)];
    assert!(trace.status.is_final);
    assert_eq!(
        trace.status.current,
        Reported::Known(ShipmentStatus::Delivered)
    );
    assert_eq!(trace.history[0].location.longitude, Some(4.5));
    assert_eq!(trace.link_tracktrace, None);
}

#[tokio::test]
async fn related_returns_use_their_content_type_and_mail_flag() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/shipments"))
        .and(header(
            "content-type",
            "application/vnd.return_shipment+json",
        ))
        .and(query_param("send_return_mail", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"data":{"shipments":[{"id":77,"reference_identifier":null}]}}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let item = RelatedReturn::new(
        ShipmentId(12345),
        ReturnCarrier::PostNl,
        "John Doe",
        EmailAddress::new("john@example.com").unwrap(),
    )
    .unwrap();
    let created = client
        .returns()
        .create_related(&NonEmpty::one(item), false)
        .await
        .unwrap();
    assert_eq!(created[0].id, ShipmentId(77));
}

#[tokio::test]
async fn webhooks_round_trip() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/webhook_subscriptions"))
        .and(header("content-type", "application/json;version=1.1"))
        .and(body_json(
            serde_json::json!({"data": {"webhook_subscriptions": [
                {"hook": "shipment_status_change", "url": "https://example.com/hook"}
            ]}}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(r#"{"data":{"ids":[{"id":9}]}}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/webhook_subscriptions"))
        .and(query_param("hook", "shipment_status_change"))
        .respond_with(respond("subscriptions"))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/webhook_subscriptions/9"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .expect(1)
        .mount(&server)
        .await;

    let url: WebhookUrl = "https://example.com/hook".parse().unwrap();
    let id = client
        .webhooks()
        .subscribe(Hook::ShipmentStatusChange, &url)
        .await
        .unwrap();
    assert_eq!(id, WebhookSubscriptionId(9));

    let listed = client
        .webhooks()
        .list(Some(Hook::ShipmentStatusChange))
        .await
        .unwrap();
    assert_eq!(listed[0].hook, Reported::Known(Hook::ShipmentStatusChange));
    assert!(listed[1].hook.known().is_none());

    client
        .webhooks()
        .unsubscribe(&NonEmpty::one(id))
        .await
        .unwrap();
}

#[tokio::test]
async fn whoami_goes_to_the_iam_host_and_an_invalid_key_is_not_an_error() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/iam/whoami"))
        .and(header("authorization", AUTH))
        .respond_with(respond("whoami"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/iam/whoami"))
        .respond_with(error(401, r#"{"message":"Access Denied"}"#))
        .mount(&server)
        .await;

    let principal = client.account().principal().await.unwrap();
    assert!(principal.has_feature(Feature::DirectPrinting));
    assert!(!client.account().is_valid().await.unwrap());
}

#[tokio::test]
async fn capabilities_are_asked_with_version_2_and_summarised() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/shipments/capabilities"))
        .and(header("content-type", "application/json;version=2.0"))
        .and(body_json(serde_json::json!({
            "recipient": {"countryCode": "NL"},
            "carrier": "POSTNL",
            "options": {"requiresSignature": {}}
        })))
        .respond_with(respond("capabilities"))
        .expect(1)
        .mount(&server)
        .await;

    let request = CapabilitiesRequest::for_country(CountryCode::NL)
        .carrier(CarrierV2::PostNl)
        .option(CapabilityOption::RequiresSignature);
    let answer = client.capabilities().get(&request).await.unwrap();

    assert_eq!(answer.collo_max(), Some(10));
    assert!(answer.carrier().is_none(), "two different carriers");
    assert_eq!(answer.package_types().len(), 3);
    let postnl = &answer.results[0];
    assert!(postnl.option(CapabilityOption::RequiresSignature).is_some());
    assert_eq!(
        postnl
            .insurance
            .as_ref()
            .unwrap()
            .insured_amount
            .as_ref()
            .unwrap()
            .max
            .as_ref()
            .unwrap()
            .amount,
        Some(500_000)
    );
    assert_eq!(postnl.physical_properties["weight"].max.value, 23000.0);
    assert!(answer.shipment_options().contains("tracked"));
    assert_eq!(Carrier::from(CarrierV2::PostNl), Carrier::PostNl);
}

#[tokio::test]
async fn contract_definitions_accept_a_carrier_filter() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/shipments/capabilities/contract-definitions"))
        .and(body_json(serde_json::json!({"carrier": "DPD"})))
        .respond_with(respond("contract_definitions"))
        .expect(1)
        .mount(&server)
        .await;
    let definitions = client
        .capabilities()
        .contract_definitions(Some(ContractCarrier::Dpd))
        .await
        .unwrap();
    assert_eq!(definitions[0].collo.max, 1);
    assert!(definitions[0].options.contains_key("hideSender"));
    assert!(!definitions[0].options["hideSender"].is_required);
}

#[tokio::test]
async fn capabilities_of_an_unknown_carrier_are_kept() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/shipments/capabilities"))
        .respond_with(respond("capabilities_unknown"))
        .mount(&server)
        .await;
    let request = CapabilitiesRequest::for_country(CountryCode::NL);
    let answer = client.capabilities().get(&request).await.unwrap();
    assert!(answer.results[0].carrier.known().is_none());
    assert_eq!(answer.carrier(), Some(&answer.results[0].carrier));
}

#[tokio::test]
async fn the_bodies_of_webhook_and_contract_definition_requests_satisfy_their_schemas() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/webhook_subscriptions"))
        .respond_with(respond("webhook_created"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/shipments/capabilities/contract-definitions"))
        .respond_with(respond("contract_definitions"))
        .mount(&server)
        .await;

    let url: WebhookUrl = "https://example.com/hook".parse().unwrap();
    for hook in Hook::ALL {
        client.webhooks().subscribe(*hook, &url).await.unwrap();
    }
    client
        .capabilities()
        .contract_definitions(None)
        .await
        .unwrap();
    for carrier in ContractCarrier::ALL {
        client
            .capabilities()
            .contract_definitions(Some(*carrier))
            .await
            .unwrap();
    }

    let spec = common::schema::core();
    for request in server.received_requests().await.unwrap() {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        let schema = if request.url.path() == "/webhook_subscriptions" {
            "webhooks_post_webhook_subscriptions_request-v1.1"
        } else {
            "capabilities_post_contract_definitions_request-v2"
        };
        common::schema::assert_valid(&spec, schema, &body);
    }
}

#[tokio::test]
async fn a_valid_key_is_recognised_whatever_the_principal_looks_like() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/iam/whoami"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"something":"else"}"#, "application/json"),
        )
        .mount(&server)
        .await;
    assert!(client.account().is_valid().await.unwrap());
    assert!(matches!(
        client.account().principal().await.unwrap_err(),
        Error::Decode { .. }
    ));
}

#[tokio::test]
async fn a_failing_key_check_is_an_error_not_a_verdict() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/iam/whoami"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let error = client.account().is_valid().await.unwrap_err();
    assert_eq!(error.as_api().map(|api| api.status), Some(503));
}
