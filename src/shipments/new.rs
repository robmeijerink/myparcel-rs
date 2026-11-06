use serde::Serialize;

use crate::enums::Carrier;
use crate::error::ValidationError;
use crate::serde_util::ser_opt_int_bool;
use crate::shipments::customs::CustomsDeclaration;
use crate::shipments::location::{DropOffPoint, Pickup};
use crate::shipments::options::ShipmentOptions;
use crate::shipments::recipient::{EmailAddress, Recipient, Sender};
use crate::types::{AccountId, ContractId, Grams, NonEmpty, ReferenceIdentifier, ShopId};

/// The size and weight of a shipment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PhysicalProperties {
    weight: Grams,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    volume: Option<u32>,
}

impl PhysicalProperties {
    /// Properties with the weight the API requires.
    pub fn new(weight: Grams) -> Self {
        Self {
            weight,
            height: None,
            width: None,
            length: None,
            volume: None,
        }
    }

    /// Sets the height.
    pub fn height(mut self, value: u32) -> Self {
        self.height = Some(value);
        self
    }

    /// Sets the width.
    pub fn width(mut self, value: u32) -> Self {
        self.width = Some(value);
        self
    }

    /// Sets the length.
    pub fn length(mut self, value: u32) -> Self {
        self.length = Some(value);
        self
    }

    /// Sets the volume.
    pub fn volume(mut self, value: u32) -> Self {
        self.volume = Some(value);
        self
    }

    /// The weight.
    pub fn weight(&self) -> Grams {
        self.weight
    }
}

/// Settings that apply to a single shipment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct GeneralSettings {
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_int_bool"
    )]
    save_recipient_address: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_int_bool"
    )]
    delivery_notification: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delivery_notification_email: Option<Vec<EmailAddress>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_int_bool"
    )]
    disable_auto_detect_pickup: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    printer_identifier: Option<String>,
}

impl GeneralSettings {
    /// Settings that change nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the recipient is saved in the address book.
    pub fn save_recipient_address(mut self, value: bool) -> Self {
        self.save_recipient_address = Some(value);
        self
    }

    /// Whether a delivery notification is sent.
    pub fn delivery_notification(mut self, value: bool) -> Self {
        self.delivery_notification = Some(value);
        self
    }

    /// Sets the addresses that receive the delivery notification.
    pub fn delivery_notification_email(mut self, value: Vec<EmailAddress>) -> Self {
        self.delivery_notification_email = Some(value);
        self
    }

    /// Whether PostNL pick-up addresses are left undetected.
    pub fn disable_auto_detect_pickup(mut self, value: bool) -> Self {
        self.disable_auto_detect_pickup = Some(value);
        self
    }

    /// Sets the printer the label is sent to.
    pub fn printer_identifier(mut self, value: impl Into<String>) -> Self {
        self.printer_identifier = Some(value.into());
        self
    }
}

/// An additional collo of a multi-collo shipment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecondaryShipment {
    reference_identifier: ReferenceIdentifier,
    carrier: Carrier,
    physical_properties: PhysicalProperties,
}

/// A shipment to create.
///
/// The recipient, the carrier and the options are required by the API, so they are required here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewShipment {
    carrier: Carrier,
    recipient: Recipient,
    options: ShipmentOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<AccountId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shop_id: Option<ShopId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract_id: Option<ContractId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference_identifier: Option<ReferenceIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sender: Option<Sender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    physical_properties: Option<PhysicalProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    customs_declaration: Option<CustomsDeclaration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pickup: Option<Pickup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drop_off_point: Option<DropOffPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    general_settings: Option<GeneralSettings>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_int_bool"
    )]
    hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secondary_shipments: Option<NonEmpty<SecondaryShipment>>,
}

impl NewShipment {
    /// A shipment with the fields the API requires.
    pub fn new(carrier: Carrier, recipient: Recipient, options: ShipmentOptions) -> Self {
        Self {
            carrier,
            recipient,
            options,
            account_id: None,
            shop_id: None,
            contract_id: None,
            reference_identifier: None,
            sender: None,
            physical_properties: None,
            customs_declaration: None,
            pickup: None,
            drop_off_point: None,
            note: None,
            general_settings: None,
            hidden: None,
            secondary_shipments: None,
        }
    }

    /// Sets the account the shipment belongs to.
    pub fn account_id(mut self, value: AccountId) -> Self {
        self.account_id = Some(value);
        self
    }

    /// Sets the shop the shipment belongs to.
    pub fn shop_id(mut self, value: ShopId) -> Self {
        self.shop_id = Some(value);
        self
    }

    /// Sets the carrier contract.
    pub fn contract_id(mut self, value: ContractId) -> Self {
        self.contract_id = Some(value);
        self
    }

    /// Sets your own reference. A shipment without one gets a generated `sdk_` reference when it
    /// is created.
    pub fn reference_identifier(mut self, value: ReferenceIdentifier) -> Self {
        self.reference_identifier = Some(value);
        self
    }

    /// Sets a sender that differs from the shop's.
    pub fn sender(mut self, value: Sender) -> Self {
        self.sender = Some(value);
        self
    }

    /// Sets the size and weight.
    pub fn physical_properties(mut self, value: PhysicalProperties) -> Self {
        self.physical_properties = Some(value);
        self
    }

    /// Sets the customs declaration.
    pub fn customs_declaration(mut self, value: CustomsDeclaration) -> Self {
        self.customs_declaration = Some(value);
        self
    }

    /// Sets the pick-up location.
    pub fn pickup(mut self, value: Pickup) -> Self {
        self.pickup = Some(value);
        self
    }

    /// Sets the drop-off point.
    pub fn drop_off_point(mut self, value: DropOffPoint) -> Self {
        self.drop_off_point = Some(value);
        self
    }

    /// Sets a note.
    pub fn note(mut self, value: impl Into<String>) -> Self {
        self.note = Some(value.into());
        self
    }

    /// Sets the general settings.
    pub fn general_settings(mut self, value: GeneralSettings) -> Self {
        self.general_settings = Some(value);
        self
    }

    /// Sets whether the shipment is hidden.
    pub fn hidden(mut self, value: bool) -> Self {
        self.hidden = Some(value);
        self
    }

    /// The country of the recipient.
    pub fn recipient_country(&self) -> crate::country::CountryCode {
        self.recipient.country()
    }

    /// The weight, if the physical properties were set.
    pub fn weight(&self) -> Option<Grams> {
        self.physical_properties
            .map(|properties| properties.weight())
    }

    /// The reference of this shipment, if one was set.
    pub fn reference(&self) -> Option<&ReferenceIdentifier> {
        self.reference_identifier.as_ref()
    }

    /// Turns the shipment into a multi-collo shipment of `colli` parcels.
    ///
    /// The weight has to be set. Every collo gets the total weight divided by `colli`, rounded
    /// down, so up to `colli - 1` grams are lost. All colli share the reference of the shipment;
    /// a shipment without one gets `multi_collo_` followed by a random suffix. The other
    /// properties of the main shipment are kept, including its dimensions.
    pub fn split_into(mut self, colli: u32) -> Result<Self, ValidationError> {
        if colli < 2 {
            return Err(ValidationError::new(
                "colli",
                "a multi-collo shipment needs at least 2 colli",
            ));
        }
        let Some(properties) = self.physical_properties else {
            return Err(ValidationError::new(
                "physical properties",
                "the weight has to be set before a shipment can be split",
            ));
        };
        let per_collo = Grams::new(properties.weight().get() / colli)?;
        let reference = match self.reference_identifier.take() {
            Some(reference) => reference,
            None => generated_reference("multi_collo_")?,
        };
        let secondary = (1..colli)
            .map(|_| SecondaryShipment {
                reference_identifier: reference.clone(),
                carrier: self.carrier,
                physical_properties: PhysicalProperties::new(per_collo),
            })
            .collect();
        self.physical_properties = Some(PhysicalProperties {
            weight: per_collo,
            ..properties
        });
        self.reference_identifier = Some(reference);
        self.secondary_shipments = Some(NonEmpty::new(secondary)?);
        Ok(self)
    }
}

pub(crate) fn generated_reference(prefix: &str) -> Result<ReferenceIdentifier, ValidationError> {
    let mut bytes = [0u8; 12];
    getrandom::fill(&mut bytes).map_err(|error| {
        ValidationError::new(
            "reference identifier",
            format!("no randomness available: {error}"),
        )
    })?;
    let suffix: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    ReferenceIdentifier::new(format!("{prefix}{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::country::CountryCode;
    use crate::enums::PackageType;
    use crate::shipments::recipient::{City, Person, PostalCode, Street};

    fn shipment() -> NewShipment {
        NewShipment::new(
            Carrier::PostNl,
            Recipient::new(
                CountryCode::NL,
                PostalCode::new("2132 JE").unwrap(),
                City::new("Hoofddorp").unwrap(),
                Street::new("Antareslaan").unwrap(),
                Person::new("John Doe").unwrap(),
            ),
            ShipmentOptions::new(PackageType::Package),
        )
    }

    #[test]
    fn splitting_divides_the_weight_and_shares_the_reference() {
        let split = shipment()
            .physical_properties(PhysicalProperties::new(Grams::new(1000).unwrap()).height(10))
            .reference_identifier(ReferenceIdentifier::new("order-1").unwrap())
            .split_into(3)
            .unwrap();
        let json = serde_json::to_value(&split).unwrap();
        assert_eq!(json["physical_properties"]["weight"], 333);
        assert_eq!(json["physical_properties"]["height"], 10);
        assert_eq!(json["secondary_shipments"].as_array().unwrap().len(), 2);
        assert_eq!(
            json["secondary_shipments"][0]["reference_identifier"],
            "order-1"
        );
        assert_eq!(
            json["secondary_shipments"][1]["physical_properties"]["weight"],
            333
        );
        assert_eq!(json["secondary_shipments"][1]["carrier"], 1);
    }

    #[test]
    fn splitting_needs_a_weight_and_two_colli() {
        assert!(shipment().split_into(2).is_err());
        let weighted =
            shipment().physical_properties(PhysicalProperties::new(Grams::new(10).unwrap()));
        assert!(weighted.clone().split_into(1).is_err());
        let split = weighted.split_into(2).unwrap();
        assert!(
            split
                .reference()
                .unwrap()
                .as_str()
                .starts_with("multi_collo_")
        );
    }
}
