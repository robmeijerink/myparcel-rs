#![cfg(feature = "connect")]

mod common;

use common::schema;
use myparcel_rs::connect::*;
use myparcel_rs::country::CountryCode;
use myparcel_rs::enums::{CarrierV2, PackageTypeV2};
use myparcel_rs::types::NonEmpty;
use serde_json::json;
use time::macros::{date, datetime};

fn address() -> OrderAddress {
    OrderAddress::new(
        OrderCity::new("Hoofddorp").unwrap(),
        CountryCode::NL,
        OrderStreet::new("Antareslaan").unwrap(),
    )
    .additional_information(AdditionalInformation::new("Ring twice").unwrap())
    .box_number(OrderBoxNumber::new("4").unwrap())
    .house_number(OrderHouseNumber::new("31").unwrap())
    .house_number_suffix(OrderHouseNumberSuffix::new("B").unwrap())
    .postal_code(OrderPostalCode::new("2132 JE").unwrap())
    .region(OrderRegion::new("Noord-Holland").unwrap())
    .state(OrderRegion::new("NH").unwrap())
}

fn money(micros: u64) -> OrderMoney {
    OrderMoney::new(CurrencyCode::new("EUR").unwrap(), micros).unwrap()
}

fn product() -> OrderLineProduct {
    OrderLineProduct::new("Socks", SourceId::new("P-1").unwrap())
        .barcode(ProductBarcode::new("123456").unwrap())
        .country_code_of_origin(CountryCode::NL)
        .descriptions(NonEmpty::one(ProductDescription::new(
            DescriptionText::new("Warm socks").unwrap(),
            LanguageCode::new("en").unwrap(),
        )))
        .ean(Ean::new("8712345678906").unwrap())
        .hs_code(HsCode::new("611595").unwrap())
        .physical_properties(
            ProductProperties::new()
                .height(Dimension::new(10, DimensionUnit::Cm).unwrap())
                .length(Dimension::new(20, DimensionUnit::Cm).unwrap())
                .width(Dimension::new(5, DimensionUnit::Mm).unwrap())
                .weight(OrderWeight::new(150, OrderWeightUnit::G).unwrap()),
        )
        .purchase_price(money(5_000_000))
        .sales_price(SalesPrice::new(money(12_950_000), 21).unwrap())
        .sku("SKU-1")
}

fn order(shipping: Shipping) -> Order {
    let business = Contact::new(ContactKind::Business {
        company: ContactCompany::new("ACME").unwrap(),
        attention: Some(ContactCompany::new("Accounting").unwrap()),
    })
    .email_address(ContactEmail::new("billing@example.com").unwrap())
    .locale(Locale::new("nl_NL").unwrap())
    .phone_number(ContactPhone::new("+31201234567").unwrap());

    Order::new(
        datetime!(2026-11-06 09:30:00 UTC),
        SourceId::new("ORDER-1").unwrap(),
        OrderStatus::Open,
    )
    .billing_details(BillingDetails {
        address: address(),
        contact: business,
    })
    .customer_reference(CustomerReference::new("customer-9").unwrap())
    .lines(NonEmpty::one(
        OrderLine::new(product(), 2, SourceId::new("L-1").unwrap())
            .unwrap()
            .price(money(25_900_000)),
    ))
    .shipping(shipping)
}

fn assert_order_valid(order: &Order) {
    schema::assert_valid(
        &schema::connect(),
        "Order",
        &serde_json::to_value(order).unwrap(),
    );
}

#[test]
fn a_fully_specified_order_satisfies_the_schema() {
    let recipient = OrderRecipient {
        address: address(),
        contact: RecipientContact::new(ContactKind::Private {
            name: ContactName::new("John Doe").unwrap(),
        })
        .email_addresses(
            NonEmpty::new(vec![
                ContactEmail::new("a@example.com").unwrap(),
                ContactEmail::new("b@example.com").unwrap(),
            ])
            .unwrap(),
        )
        .locale(Locale::new("nl").unwrap())
        .phone_number(ContactPhone::new("0612345678").unwrap()),
    };
    let mut options = OrderShipmentOptions::new()
        .insurance(money(100_000_000))
        .return_contribution_fee(money(2_000_000))
        .custom_label_text(CustomLabelText::new("Handle with care").unwrap());
    for option in OrderOption::ALL {
        options = options.flag(*option);
    }
    let shipping = Shipping::new()
        .additional_context(AdditionalContext::new("Leave at the neighbours").unwrap())
        .carrier(CarrierV2::PostNl)
        .delivery(
            DeliverySettings::new()
                .delivery_type(OrderDeliveryType::Standard)
                .notify_on_delivery()
                .preferred_date(date!(2026 - 11 - 10)),
        )
        .options(options)
        .package_type(PackageTypeV2::Package)
        .recipient(recipient);
    assert_order_valid(&order(shipping));
}

#[test]
fn every_pickup_kind_satisfies_the_schema() {
    let spot = || {
        PickupSpot::new(PickupName::new("Corner shop").unwrap())
            .code(PickupCode::new("C-1").unwrap())
    };
    for kind in [
        PickupKind::General,
        PickupKind::PostNl(RetailNetwork::Netherlands),
        PickupKind::PostNl(RetailNetwork::Belgium),
        PickupKind::Bpost(BpostPickupType::ClickCollect),
        PickupKind::Bpost(BpostPickupType::PostPoint),
    ] {
        let pickup = OrderPickup {
            address: address(),
            location: spot(),
            kind,
        };
        assert_order_valid(&order(
            Shipping::new().carrier(CarrierV2::PostNl).pickup(pickup),
        ));
    }
}

#[test]
fn every_delivery_and_package_type_satisfies_the_schema() {
    for delivery in OrderDeliveryType::ALL {
        let shipping = Shipping::new().delivery(DeliverySettings::new().delivery_type(*delivery));
        assert_order_valid(&order(shipping));
    }
    for package in PackageTypeV2::ALL {
        assert_order_valid(&order(Shipping::new().package_type(*package)));
    }
    for carrier in CarrierV2::ALL {
        assert_order_valid(&order(Shipping::new().carrier(*carrier)));
    }
}

#[test]
fn the_smallest_order_and_a_canceled_order_satisfy_the_schema() {
    for status in OrderStatus::ALL {
        let order = Order::new(
            datetime!(2026-01-01 00:00:00 UTC),
            SourceId::new("X").unwrap(),
            *status,
        );
        assert_order_valid(&order);
    }
}

#[test]
fn a_batch_satisfies_the_request_body_schema() {
    let document = schema::connect();
    let body =
        document["paths"]["/webhook/orders"]["post"]["requestBody"]["content"]["application/json"]
            ["schema"]
            .clone();
    let orders =
        NonEmpty::<Order, 50>::new(vec![order(Shipping::new()), order(Shipping::new())]).unwrap();
    schema::assert_valid_inline(&document, &body, &serde_json::to_value(&orders).unwrap());
    schema::assert_valid_inline(
        &document,
        &body,
        &json!([serde_json::to_value(order(Shipping::new())).unwrap()]),
    );
}
