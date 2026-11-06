//! Runs against the real API with a throwaway concept shipment.
//!
//! `MYPARCEL_API_KEY=... cargo test --test live -- --ignored`

use myparcel_rs::country::CountryCode;
use myparcel_rs::enums::{Carrier, PackageType, ShipmentStatus};
use myparcel_rs::shipments::{
    City, HouseNumber, NewShipment, Person, PostalCode, Recipient, ShipmentBatch, ShipmentOptions,
    Street,
};
use myparcel_rs::types::{ReferenceIdentifier, ShipmentIds};
use myparcel_rs::{ApiKey, Client, Reported};

#[tokio::test]
#[ignore = "needs MYPARCEL_API_KEY"]
async fn a_concept_can_be_created_found_and_deleted() {
    let key =
        ApiKey::new(std::env::var("MYPARCEL_API_KEY").expect("MYPARCEL_API_KEY is set")).unwrap();
    let client = Client::new(key).unwrap();

    assert!(client.account().is_valid().await.unwrap());

    let reference =
        ReferenceIdentifier::new(format!("myparcel-rs-live-{}", std::process::id())).unwrap();
    let recipient = Recipient::new(
        CountryCode::NL,
        PostalCode::new("2132 JE").unwrap(),
        City::new("Hoofddorp").unwrap(),
        Street::new("Antareslaan").unwrap(),
        Person::new("myparcel-rs live test").unwrap(),
    )
    .number(HouseNumber::new("31").unwrap());
    let shipment = NewShipment::new(
        Carrier::PostNl,
        recipient,
        ShipmentOptions::new(PackageType::Package),
    )
    .reference_identifier(reference.clone());

    let created = client
        .shipments()
        .create(&ShipmentBatch::one(shipment), None)
        .await
        .unwrap();
    let ids = ShipmentIds::new(
        created
            .shipments
            .iter()
            .map(|shipment| shipment.id)
            .collect(),
    )
    .unwrap();

    let found = client
        .shipments()
        .find_by_reference(&reference)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].status,
        Some(Reported::Known(ShipmentStatus::PendingConcept))
    );

    client.shipments().delete(&ids).await.unwrap();
}
