//! Shipments as the API reports them.

use std::collections::BTreeMap;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use crate::enums::{
    Carrier, DeliveryType, PackageType, ShipmentStatus, ShipmentType, TransactionStatus,
};
use crate::reported::Reported;
use crate::serde_util::{opt_amount, opt_string_or_int};
use crate::shipments::options::ShipmentOption;
use crate::types::{AccountId, ContractId, ShipmentId, ShopId};

/// An amount of money reported by the API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Price {
    /// The amount in the lowest denomination of the currency, e.g. cents.
    #[serde(default, deserialize_with = "opt_amount")]
    pub amount: Option<u64>,
    /// The currency; empty when the API does not know it.
    pub currency: String,
}

/// An address as reported by the API.
///
/// Everything is optional and unvalidated: the API reports addresses that it would not accept as
/// input.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Address {
    /// The country code.
    pub cc: Option<String>,
    /// The name of the person.
    pub person: Option<String>,
    /// The first name.
    pub first_name: Option<String>,
    /// The last name.
    pub last_name: Option<String>,
    /// The company.
    pub company: Option<String>,
    /// The street.
    pub street: Option<String>,
    /// Additional street information.
    pub street_additional_info: Option<String>,
    /// The house number.
    pub number: Option<String>,
    /// The house number suffix.
    pub number_suffix: Option<String>,
    /// The box number.
    pub box_number: Option<String>,
    /// The postal code.
    pub postal_code: Option<String>,
    /// The city.
    pub city: Option<String>,
    /// The region.
    pub region: Option<String>,
    /// The state.
    pub state: Option<String>,
    /// The area.
    pub area: Option<String>,
    /// The phone number.
    pub phone: Option<String>,
    /// The email address.
    pub email: Option<String>,
}

/// The options of a shipment as reported by the API.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Options {
    /// The package type.
    pub package_type: Option<Reported<PackageType>>,
    /// The delivery type.
    pub delivery_type: Option<Reported<DeliveryType>>,
    /// The delivery date, as the API formats it.
    pub delivery_date: Option<String>,
    /// The label description.
    pub label_description: Option<String>,
    /// The insured amount.
    pub insurance: Option<Price>,
    /// The yes/no options the API reported.
    pub flags: BTreeMap<ShipmentOption, bool>,
}

impl Options {
    /// Whether an option is switched on, if the API reported it.
    pub fn flag(&self, option: ShipmentOption) -> Option<bool> {
        self.flags.get(&option).copied()
    }
}

fn reported<T: serde::de::DeserializeOwned, E: serde::de::Error>(
    value: Value,
) -> Result<Reported<T>, E> {
    Reported::deserialize(value).map_err(E::custom)
}

impl<'de> Deserialize<'de> for Options {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut map = Map::deserialize(deserializer)?;
        let mut take = |name: &str| map.remove(name).filter(|value| !value.is_null());
        let text = |value: Value| value.as_str().map(str::to_owned);

        let package_type = take("package_type")
            .map(reported::<_, D::Error>)
            .transpose()?;
        let delivery_type = take("delivery_type")
            .map(reported::<_, D::Error>)
            .transpose()?;
        let delivery_date = take("delivery_date").and_then(text);
        let label_description = take("label_description").and_then(text);
        let insurance = take("insurance")
            .map(|value| Price::deserialize(value).map_err(D::Error::custom))
            .transpose()?;

        let mut flags = BTreeMap::new();
        for option in ShipmentOption::ALL {
            let Some(value) = take(option.as_str()) else {
                continue;
            };
            let enabled = match &value {
                Value::Bool(enabled) => *enabled,
                Value::Number(number) if number.as_u64() == Some(0) => false,
                Value::Number(number) if number.as_u64() == Some(1) => true,
                other => {
                    return Err(D::Error::custom(format!(
                        "option {option} is {other}, expected 0 or 1"
                    )));
                }
            };
            flags.insert(*option, enabled);
        }

        Ok(Self {
            package_type,
            delivery_type,
            delivery_date,
            label_description,
            insurance,
            flags,
        })
    }
}

/// The pick-up location of a shipment as reported by the API.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Pickup {
    /// The location code.
    pub location_code: Option<String>,
    /// The name of the location.
    pub location_name: Option<String>,
    /// The street.
    pub street: Option<String>,
    /// The house number.
    pub number: Option<String>,
    /// The house number suffix.
    pub number_suffix: Option<String>,
    /// The box number.
    pub box_number: Option<String>,
    /// The postal code.
    pub postal_code: Option<String>,
    /// The city.
    pub city: Option<String>,
    /// The region.
    pub region: Option<String>,
    /// The state.
    pub state: Option<String>,
    /// The country code.
    pub cc: Option<String>,
    /// The kind of location.
    pub location_type: Option<Reported<crate::shipments::PickupLocationType>>,
    /// The retail network id.
    pub retail_network_id: Option<String>,
}

/// The size and weight of a shipment as reported by the API.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct PhysicalProperties {
    /// The weight in grams.
    pub weight: Option<u64>,
    /// The height.
    pub height: Option<u64>,
    /// The width.
    pub width: Option<u64>,
    /// The length.
    pub length: Option<u64>,
    /// The volume.
    pub volume: Option<u64>,
}

/// A shipment as reported by the API.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Shipment {
    /// The id.
    pub id: ShipmentId,
    /// The id of the parent shipment, for return and multi-collo shipments.
    pub parent_id: Option<ShipmentId>,
    /// The account.
    pub account_id: Option<AccountId>,
    /// The shop.
    pub shop_id: Option<ShopId>,
    /// The carrier contract.
    pub contract_id: Option<ContractId>,
    /// The type of shipment.
    pub shipment_type: Option<Reported<ShipmentType>>,
    /// The carrier.
    pub carrier_id: Option<Reported<Carrier>>,
    /// The status.
    pub status: Option<Reported<ShipmentStatus>>,
    /// Whether the shipment has been paid for.
    pub transaction_status: Option<Reported<TransactionStatus>>,
    /// Your reference.
    #[serde(default, deserialize_with = "opt_string_or_int")]
    pub reference_identifier: Option<String>,
    /// The carrier barcode, once registered.
    pub barcode: Option<String>,
    /// The recipient.
    pub recipient: Option<Address>,
    /// The sender.
    pub sender: Option<Address>,
    /// The options.
    pub options: Option<Options>,
    /// The pick-up location.
    pub pickup: Option<Pickup>,
    /// The size and weight.
    pub physical_properties: Option<PhysicalProperties>,
    /// The price.
    pub price: Option<Price>,
    /// The region.
    pub region: Option<String>,
    /// When the shipment was created, as the API formats it.
    pub created: Option<String>,
    /// When the shipment was last modified, as the API formats it.
    pub modified: Option<String>,
    /// Whether the shipment is delayed.
    pub delayed: Option<bool>,
    /// Whether the shipment is delivered.
    pub delivered: Option<bool>,
    /// The consumer portal link, empty when there is none.
    pub link_consumer_portal: Option<String>,
    /// The main shipment of a multi-collo shipment.
    pub multi_collo_main_shipment_id: Option<ShipmentId>,
    /// The other colli of a multi-collo shipment.
    pub secondary_shipments: Option<Vec<Shipment>>,
    /// The identifier at the carrier.
    pub external_identifier: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_options_and_flags() {
        let options: Options = serde_json::from_str(
            r#"{"package_type":1,"delivery_type":2,"signature":1,"only_recipient":0,"insurance":{"amount":"5000","currency":"EUR"},"unknown_option":1}"#,
        )
        .unwrap();
        assert_eq!(
            options.package_type,
            Some(Reported::Known(PackageType::Package))
        );
        assert_eq!(options.flag(ShipmentOption::Signature), Some(true));
        assert_eq!(options.flag(ShipmentOption::OnlyRecipient), Some(false));
        assert_eq!(options.flag(ShipmentOption::Frozen), None);
        assert_eq!(options.insurance.unwrap().amount, Some(5000));
    }

    #[test]
    fn keeps_values_it_does_not_know() {
        let shipment: Shipment = serde_json::from_str(
            r#"{"id":5,"reference_identifier":12,"status":4242,"carrier_id":99,"options":{"package_type":1}}"#,
        )
        .unwrap();
        assert_eq!(shipment.reference_identifier.as_deref(), Some("12"));
        assert_eq!(shipment.status, Some(Reported::Unknown(Value::from(4242))));
        assert_eq!(
            shipment.carrier_id,
            Some(Reported::Unknown(Value::from(99)))
        );
    }

    #[test]
    fn rejects_non_boolean_flags() {
        assert!(serde_json::from_str::<Options>(r#"{"signature":5}"#).is_err());
    }
}
