mod common;

use std::collections::BTreeSet;

use common::schema;

use myparcel_rs::capabilities::{
    CapabilitiesRequest, CapabilityOption, CapabilityProperties, InsuranceRequest, Length,
    LengthUnit, Weight, WeightUnit,
};
use myparcel_rs::country::CountryCode;
use myparcel_rs::enums::*;
use myparcel_rs::returns::{
    RelatedReturn, ReturnCarrier, ReturnOption, ReturnOptions, ReturnPackageType, ReturnSender,
    UnrelatedReturn,
};
use myparcel_rs::shipments::*;
use myparcel_rs::types::*;
use serde_json::{Value, json};

fn spec() -> Value {
    schema::core()
}

fn assert_valid(spec: &Value, name: &str, instance: &Value) {
    schema::assert_valid(spec, name, instance);
}

fn spec_ints(spec: &Value, name: &str) -> BTreeSet<i64> {
    spec["components"]["schemas"][name]["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("{name} has no enum"))
        .iter()
        .map(|value| value.as_i64().unwrap())
        .collect()
}

fn spec_strings(spec: &Value, name: &str) -> BTreeSet<String> {
    spec["components"]["schemas"][name]["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("{name} has no enum"))
        .iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect()
}

fn ints<T: Copy>(all: &[T], value: impl Fn(T) -> i64) -> BTreeSet<i64> {
    all.iter().copied().map(value).collect()
}

fn strings<T: Copy>(all: &[T], value: impl Fn(T) -> &'static str) -> BTreeSet<String> {
    all.iter()
        .copied()
        .map(|item| value(item).to_owned())
        .collect()
}

#[test]
fn integer_enums_match_the_spec() {
    let spec = spec();
    assert_eq!(
        ints(Carrier::ALL, |v| v.value().into()),
        spec_ints(&spec, "_ref_types_carrier")
    );
    assert_eq!(
        ints(ReturnCarrier::ALL, |v| v.value().into()),
        spec_ints(&spec, "_ref_types_carrier_returns")
    );
    assert_eq!(
        ints(PackageType::ALL, |v| v.value().into()),
        spec_ints(&spec, "_ref_shipment_package_type")
    );
    assert_eq!(
        ints(DeliveryType::ALL, |v| v.value().into()),
        spec_ints(&spec, "_ref_types_delivery_type")
    );
    assert_eq!(
        ints(ShipmentStatus::ALL, |v| v.value().into()),
        spec_ints(&spec, "shipment_defs_shipment_status")
    );
    assert_eq!(
        ints(LabelQuadrant::ALL, |v| v.value().into()),
        spec_ints(&spec, "_ref_shipment_label_printing_position")
    );
    assert_eq!(
        ints(CustomsContents::ALL, |v| v.value().into()),
        spec_ints(&spec, "shipment_defs_customs_shipment_type")
    );
    assert_eq!(
        ints(ShipmentType::ALL, |v| v.value().into()),
        spec_ints(&spec, "_ref_shipment_type")
    );
}

#[test]
fn string_enums_match_the_spec() {
    let spec = spec();
    assert_eq!(
        strings(CarrierV2::ALL, CarrierV2::as_str),
        spec_strings(&spec, "_ref_capabilities_shared_carrier-v2")
    );
    assert_eq!(
        strings(PackageTypeV2::ALL, PackageTypeV2::as_str),
        spec_strings(&spec, "_ref_shipment_package_type-v2")
    );
    assert_eq!(
        strings(DeliveryTypeV2::ALL, DeliveryTypeV2::as_str),
        spec_strings(&spec, "_ref_types_delivery_type-v2")
    );
    assert_eq!(
        strings(TransactionStatus::ALL, TransactionStatus::as_str),
        spec_strings(&spec, "_ref_shipment_transaction_status")
    );
    assert_eq!(
        strings(TransactionType::ALL, TransactionType::as_str),
        spec_strings(&spec, "_ref_types_transaction_types")
    );
    assert_eq!(
        strings(Order::ALL, Order::as_str),
        spec_strings(&spec, "common_parameters_order")
    );
    assert_eq!(
        strings(RegionFilter::ALL, RegionFilter::as_str),
        spec_strings(&spec, "shipment_parameters_region")
    );
    assert_eq!(
        strings(Sort::ALL, Sort::as_str),
        spec_strings(&spec, "shipment_parameters_sort_shipment")
    );
    let main: BTreeSet<String> = spec_strings(&spec, "shipment_defs_main_status");
    assert_eq!(strings(MainStatus::ALL, MainStatus::as_str), main);
}

#[test]
fn legacy_and_v2_names_agree_with_the_spec() {
    let spec = spec();
    let legacy = spec_strings(&spec, "shipment_parameters_carrier_name");
    let ours: BTreeSet<String> = Carrier::ALL
        .iter()
        .map(|c| c.legacy_name().to_owned())
        .collect();
    assert_eq!(ours, legacy);
    for carrier in Carrier::ALL {
        if let Some(name) = carrier.v2_name() {
            assert_eq!(Carrier::from_v2_name(name), Some(*carrier));
        }
        assert_eq!(
            Carrier::from_legacy_name(carrier.legacy_name()),
            Some(*carrier)
        );
    }
}

fn full_shipment() -> NewShipment {
    let recipient = Recipient::new(
        CountryCode::new("DE").unwrap(),
        PostalCode::new("10115").unwrap(),
        City::new("Berlin").unwrap(),
        Street::new("Invalidenstrasse").unwrap(),
        Person::new("Erika Mustermann").unwrap(),
    )
    .number(HouseNumber::new("12").unwrap())
    .number_suffix(NumberSuffix::new("a").unwrap())
    .box_number(BoxNumber::new("3").unwrap())
    .region(Region::new("Berlin").unwrap())
    .state(State::new("BE").unwrap())
    .area(Area::new("Mitte").unwrap())
    .company(Company::new("Muster GmbH").unwrap())
    .email(EmailAddress::new("erika@example.com").unwrap())
    .secondary_emails(NonEmpty::new(vec![EmailAddress::new("b@example.com").unwrap()]).unwrap())
    .phone(Phone::new("+4930123456").unwrap())
    .eori_number("DE123")
    .vat_number("DE456");

    let options = ShipmentOptions::new(PackageType::Package)
        .flag(ShipmentOption::Signature, true)
        .flag(ShipmentOption::OnlyRecipient, true)
        .insurance(Insurance::new(Cents::from_euros(500).unwrap()).unwrap())
        .cash_on_delivery(Money::eur(Cents::new(1000)))
        .delivery_type(DeliveryType::Standard)
        .delivery_date("2026-11-10 10:00:00".parse().unwrap())
        .label_description(LabelDescription::new("Order 42").unwrap())
        .incoterms(IncotermsRule::Ddp);

    let customs = CustomsDeclaration::new(
        CustomsContents::Gift,
        Grams::new(500).unwrap(),
        NonEmpty::new(vec![
            CustomsItem::new(
                "Socks",
                ItemQuantity::new(2).unwrap(),
                Grams::new(250).unwrap(),
                Money::eur(Cents::new(1250)),
                "611595",
            )
            .unwrap()
            .country(CountryCode::NL)
            .external_product_reference(ExternalProductReference::new("sku-1").unwrap()),
        ])
        .unwrap(),
    )
    .invoice(InvoiceNumber::new("INV-1").unwrap())
    .eori_number("NL1")
    .vat_number("NL2")
    .vat_number_uk("GB3")
    .voec_number("IM4");

    NewShipment::new(Carrier::PostNl, recipient, options)
        .account_id(AccountId(1))
        .shop_id(ShopId(2))
        .contract_id(ContractId(3))
        .reference_identifier(ReferenceIdentifier::new("order-42").unwrap())
        .sender(
            Sender::new(
                CountryCode::NL,
                "2132JE",
                "Hoofddorp",
                "Antareslaan",
                "Warehouse",
            )
            .unwrap()
            .number("31")
            .email(EmailAddress::new("wh@example.com").unwrap()),
        )
        .physical_properties(
            PhysicalProperties::new(Grams::new(900).unwrap())
                .height(10)
                .width(20)
                .length(30),
        )
        .customs_declaration(customs)
        .pickup(
            Pickup::new(
                CountryCode::NL,
                "2132JE",
                "Hoofddorp",
                "Antareslaan",
                "31",
                "Shop",
            )
            .location_code("abc")
            .location_type(PickupLocationType::Retail)
            .retail_network_id("PNPNL-01"),
        )
        .drop_off_point(
            DropOffPoint::new("code", "name", "2132JE", "Hoofddorp", "Antareslaan", "31")
                .country(CountryCode::NL),
        )
        .note("fragile")
        .general_settings(
            GeneralSettings::new()
                .save_recipient_address(true)
                .delivery_notification(true)
                .disable_auto_detect_pickup(false)
                .printer_identifier("printer-1"),
        )
        .hidden(false)
}

#[test]
fn a_fully_specified_shipment_satisfies_the_create_schema() {
    let spec = spec();
    let body = json!({"data": {"user_agent": "x", "shipments": [full_shipment()]}});
    assert_valid(&spec, "shipment_post_shipments_request-v1.1", &body);
}

#[test]
fn a_split_shipment_satisfies_the_create_schema() {
    let spec = spec();
    let split = full_shipment().split_into(3).unwrap();
    let body = json!({"data": {"shipments": [split]}});
    assert_valid(&spec, "shipment_post_shipments_request-v1.1", &body);
}

#[test]
fn bpost_pickup_types_satisfy_the_schema() {
    let spec = spec();
    let pickup = Pickup::new(CountryCode::BE, "1000", "Brussel", "Rue", "1", "Point")
        .location_type(PickupLocationType::Bpost(BpostLocationType::ParcelLocker));
    let shipment = full_shipment().pickup(pickup);
    assert_valid(
        &spec,
        "shipment_post_shipments_request-v1.1",
        &json!({"data": {"shipments": [shipment]}}),
    );
}

#[test]
fn return_shipments_satisfy_their_schemas() {
    let spec = spec();
    let related = RelatedReturn::new(
        ShipmentId(12345),
        ReturnCarrier::PostNl,
        "John Doe",
        EmailAddress::new("john@example.com").unwrap(),
    )
    .unwrap()
    .account_id(AccountId(1))
    .shop_id(ShopId(2))
    .contract_id(ContractId(3))
    .reference_identifier(ReferenceIdentifier::new("ret-1").unwrap())
    .sender(
        ReturnSender::new(CountryCode::NL, "Hoofddorp", "Antareslaan", "Warehouse")
            .unwrap()
            .postal_code("2132JE")
            .company("ACME"),
    )
    .physical_properties(PhysicalProperties::new(Grams::new(300).unwrap()))
    .save_recipient_address(true)
    .options(
        ReturnOptions::new()
            .package_type(ReturnPackageType::Mailbox)
            .flag(ReturnOption::Signature, true)
            .printerless_return(true)
            .insurance(Insurance::new(Cents::new(5000)).unwrap())
            .contribution(Money::eur(Cents::new(100_000)))
            .unwrap()
            .label_description(LabelDescription::new("Return").unwrap())
            .incoterms(IncotermsRule::Dap),
    );
    assert_valid(
        &spec,
        "shipment_post_return_shipments_request",
        &json!({"data": {"return_shipments": [related]}}),
    );

    let unrelated = UnrelatedReturn::new(ReturnCarrier::Dpd)
        .shop_id(ShopId(2))
        .email(EmailAddress::new("john@example.com").unwrap())
        .name("John Doe")
        .country(CountryCode::NL)
        .options(ReturnOptions::new().package_type(ReturnPackageType::Package));
    assert_valid(
        &spec,
        "shipment_post_unrelated_return_shipments_request",
        &json!({"data": {"return_shipments": [unrelated]}}),
    );
}

#[test]
fn capability_requests_satisfy_the_v2_schema() {
    let spec = spec();
    let request = CapabilitiesRequest::for_country(CountryCode::NL)
        .shop_id(ShopId(1))
        .contract_id(ContractId(2))
        .sender(myparcel_rs::capabilities::Party::new(CountryCode::NL).is_business(true))
        .carrier(CarrierV2::PostNl)
        .package_type(PackageType::Package)
        .delivery_type(DeliveryType::Morning)
        .direction(Direction::Outbound)
        .physical_properties(
            CapabilityProperties::new()
                .weight(Weight::new(2000, WeightUnit::G))
                .height(Length::new(10, LengthUnit::Cm).unwrap()),
        )
        .option(CapabilityOption::RequiresSignature)
        .option(CapabilityOption::RecipientOnlyDelivery)
        .insurance(InsuranceRequest::Amount(Cents::new(10_000)))
        .pickup_location(LocationType::Retail);
    assert_valid(
        &spec,
        "capabilities_post_capabilities_request-v2",
        &serde_json::to_value(&request).unwrap(),
    );

    let minimal =
        CapabilitiesRequest::for_country(CountryCode::BE).insurance(InsuranceRequest::Default);
    assert_valid(
        &spec,
        "capabilities_post_capabilities_request-v2",
        &serde_json::to_value(&minimal).unwrap(),
    );
}

#[test]
fn every_capability_option_exists_in_the_spec() {
    let spec = spec();
    let schema = &spec["components"]["schemas"]["capabilities_post_capabilities_request-v2"];
    let options = &schema["properties"]["options"]["properties"];
    let spec_names: BTreeSet<String> = options.as_object().unwrap().keys().cloned().collect();
    let ours: BTreeSet<String> = CapabilityOption::ALL
        .iter()
        .map(|o| o.as_str().to_owned())
        .collect();
    assert_eq!(ours, spec_names);
}

#[test]
fn shipment_options_cover_the_spec_flags() {
    let spec = spec();
    let flags = &spec["components"]["schemas"]["_ref_shipment_options_options"]["properties"];
    let spec_names: BTreeSet<String> = flags
        .as_object()
        .unwrap()
        .keys()
        .filter(|name| name.as_str() != "cash_on_delivery")
        .cloned()
        .collect();
    let ours: BTreeSet<String> = ShipmentOption::ALL
        .iter()
        .map(|o| o.as_str().to_owned())
        .collect();
    assert_eq!(ours, spec_names);
}

fn fixture_json(name: &str) -> Value {
    serde_json::from_str(&common::fixture(name)).unwrap()
}

#[test]
fn the_response_fixtures_satisfy_the_response_schemas() {
    let spec = spec();
    for (fixture, schema_name) in [
        (
            "shipments_created",
            "shipment_responses_post_shipments-v1.2",
        ),
        ("shipments", "shipment_responses_shipments"),
        ("tracktraces", "shipment_responses_tracktraces"),
        (
            "subscriptions",
            "webhooks_responses_webhook_subscriptions-v1.1",
        ),
        (
            "webhook_created",
            "webhooks_responses_post_webhook_subscriptions",
        ),
        ("capabilities", "capabilities_responses_capabilities-v2"),
        ("label_link", "common_responses_download_link"),
        (
            "payment_required",
            "shipment_responses_payment_instructions",
        ),
        (
            "labels_held_back",
            "shipment_responses_bulk_shipment_labels_payment_required",
        ),
    ] {
        assert_valid(&spec, schema_name, &fixture_json(fixture));
    }

    // The schema of a contract definition option is unsatisfiable (`additionalProperties: false`
    // next to the properties it refers to), so only the part around the options can be checked.
    let mut definitions = fixture_json("contract_definitions");
    definitions["items"][0]["options"] = json!({});
    assert_valid(
        &spec,
        "capabilities_responses_contract-definitions-v2",
        &definitions,
    );
}

#[test]
fn the_validator_rejects_what_it_should() {
    let spec = spec();
    let post = schema::validator(&spec, "shipment_responses_post_shipments-v1.2");
    assert!(!post.is_valid(&json!({})));
    assert!(!post.is_valid(&json!({"data": {"shipments": "nope"}})));
    assert!(!post.is_valid(&json!({"data": {"shipments": [{"id": 1}]}})));

    let request = schema::validator(&spec, "shipment_post_shipments_request-v1.1");
    assert!(!request.is_valid(&json!({"data": {"shipments": []}})));
    assert!(!request.is_valid(&json!({"data": {"shipments": [{"carrier": 1}]}})));

    let order = schema::validator(&schema::connect(), "Order");
    assert!(!order.is_valid(&json!({"sourceId": "x"})));
}
