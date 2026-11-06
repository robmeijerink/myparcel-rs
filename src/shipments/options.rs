use std::collections::BTreeMap;

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use crate::enums::{DeliveryType, PackageType};
use crate::types::{DateTime, Insurance, LabelDescription, Money};

string_enum! {
    /// A yes/no option of a shipment, named as the shipments API names it.
    pub enum ShipmentOption {
        /// The recipient has to be 18 or older and sign.
        AgeCheck = "age_check",
        /// The shipment is collected at the sender.
        Collect = "collect",
        /// The shipment is delivered cooled.
        CooledDelivery = "cooled_delivery",
        /// The shipment is delivered at a postal point.
        DropOffAtPostalPoint = "drop_off_at_postal_point",
        /// Extra assurance.
        ExtraAssurance = "extra_assurance",
        /// The shipment contains fresh food.
        FreshFood = "fresh_food",
        /// The shipment is frozen.
        Frozen = "frozen",
        /// The sender is hidden on the label.
        HideSender = "hide_sender",
        /// The shipment is large format.
        LargeFormat = "large_format",
        /// The shipment is not tracked.
        NoTracking = "no_tracking",
        /// Only the recipient may receive the shipment.
        OnlyRecipient = "only_recipient",
        /// The shipment gets priority delivery.
        PriorityDelivery = "priority_delivery",
        /// The recipient needs a receipt code.
        ReceiptCode = "receipt_code",
        /// The shipment is returned when the recipient is absent.
        Return = "return",
        /// The shipment is delivered the same day.
        SameDayDelivery = "same_day_delivery",
        /// The shipment is delivered on Saturday.
        SaturdayDelivery = "saturday_delivery",
        /// The recipient has to sign.
        Signature = "signature",
        /// The shipment is tracked.
        Tracked = "tracked",
    }
}

impl ShipmentOption {
    /// The name of the matching option of the capabilities API, if it has one.
    ///
    /// `cooled_delivery` and `tracked` have no counterpart.
    pub const fn capabilities_name(self) -> Option<&'static str> {
        match self {
            Self::AgeCheck => Some("requiresAgeVerification"),
            Self::Collect => Some("scheduledCollection"),
            Self::CooledDelivery => None,
            Self::DropOffAtPostalPoint => Some("deliverAtPostalPoint"),
            Self::ExtraAssurance => Some("additionalInsurance"),
            Self::FreshFood => Some("freshFood"),
            Self::Frozen => Some("frozen"),
            Self::HideSender => Some("hideSender"),
            Self::LargeFormat => Some("oversizedPackage"),
            Self::NoTracking => Some("noTracking"),
            Self::OnlyRecipient => Some("recipientOnlyDelivery"),
            Self::PriorityDelivery => Some("priorityDelivery"),
            Self::ReceiptCode => Some("requiresReceiptCode"),
            Self::Return => Some("returnOnFirstFailedDelivery"),
            Self::SameDayDelivery => Some("sameDayDelivery"),
            Self::SaturdayDelivery => Some("saturdayDelivery"),
            Self::Signature => Some("requiresSignature"),
            Self::Tracked => None,
        }
    }
}

string_enum! {
    /// Who pays import duties and taxes of an international shipment.
    pub enum IncotermsRule {
        /// Delivered at place: duties, taxes and clearance are paid by the recipient.
        Dap = "DAP",
        /// Delivered at place, cleared: clearance is paid by the sender, duties and taxes by the recipient.
        DapCleared = "DAP_CLEARED",
        /// Delivered duty paid: duties, taxes and clearance are paid by the sender.
        Ddp = "DDP",
        /// Delivered duty paid, VAT unpaid: duties and clearance are paid by the sender, import VAT by the recipient.
        DdpVatUnpaid = "DDP_VAT_UNPAID",
    }
}

/// The options of a shipment.
///
/// The package type is required by the API and therefore by the constructor. Which options are
/// allowed for which carrier and package type is decided by the API; use the capabilities
/// service to find out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipmentOptions {
    package_type: PackageType,
    flags: BTreeMap<ShipmentOption, bool>,
    insurance: Option<Insurance>,
    cash_on_delivery: Option<Money>,
    delivery_type: Option<DeliveryType>,
    delivery_date: Option<DateTime>,
    label_description: Option<LabelDescription>,
    incoterms: Option<IncotermsRule>,
}

impl ShipmentOptions {
    /// Options for a package type.
    pub fn new(package_type: PackageType) -> Self {
        Self {
            package_type,
            flags: BTreeMap::new(),
            insurance: None,
            cash_on_delivery: None,
            delivery_type: None,
            delivery_date: None,
            label_description: None,
            incoterms: None,
        }
    }

    /// Switches an option on or off.
    pub fn flag(mut self, option: ShipmentOption, enabled: bool) -> Self {
        self.flags.insert(option, enabled);
        self
    }

    /// Sets the insured amount.
    pub fn insurance(mut self, value: Insurance) -> Self {
        self.insurance = Some(value);
        self
    }

    /// Sets the cash on delivery amount.
    pub fn cash_on_delivery(mut self, value: Money) -> Self {
        self.cash_on_delivery = Some(value);
        self
    }

    /// Sets the delivery type.
    pub fn delivery_type(mut self, value: DeliveryType) -> Self {
        self.delivery_type = Some(value);
        self
    }

    /// Sets the delivery date.
    pub fn delivery_date(mut self, value: DateTime) -> Self {
        self.delivery_date = Some(value);
        self
    }

    /// Sets the label description.
    pub fn label_description(mut self, value: LabelDescription) -> Self {
        self.label_description = Some(value);
        self
    }

    /// Sets the Incoterms rule.
    pub fn incoterms(mut self, value: IncotermsRule) -> Self {
        self.incoterms = Some(value);
        self
    }

    /// The package type.
    pub fn package_type(&self) -> PackageType {
        self.package_type
    }
}

impl Serialize for ShipmentOptions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Incoterms {
            rule: IncotermsRule,
        }

        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("package_type", &self.package_type)?;
        for (option, enabled) in &self.flags {
            map.serialize_entry(option.as_str(), &u8::from(*enabled))?;
        }
        if let Some(value) = &self.insurance {
            map.serialize_entry("insurance", value)?;
        }
        if let Some(value) = &self.cash_on_delivery {
            map.serialize_entry("cash_on_delivery", value)?;
        }
        if let Some(value) = &self.delivery_type {
            map.serialize_entry("delivery_type", value)?;
        }
        if let Some(value) = &self.delivery_date {
            map.serialize_entry("delivery_date", value)?;
        }
        if let Some(value) = &self.label_description {
            map.serialize_entry("label_description", value)?;
        }
        if let Some(rule) = self.incoterms {
            map.serialize_entry("incoterms", &Incoterms { rule })?;
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Cents;

    #[test]
    fn serializes_flags_as_integers() {
        let options = ShipmentOptions::new(PackageType::Package)
            .flag(ShipmentOption::Signature, true)
            .flag(ShipmentOption::OnlyRecipient, false)
            .insurance(Insurance::new(Cents::from_euros(250).unwrap()).unwrap())
            .delivery_type(DeliveryType::Standard);
        assert_eq!(
            serde_json::to_string(&options).unwrap(),
            r#"{"package_type":1,"only_recipient":0,"signature":1,"insurance":{"amount":25000,"currency":"EUR"},"delivery_type":2}"#
        );
    }

    #[test]
    fn capabilities_names_map_to_the_option_names() {
        assert_eq!(
            ShipmentOption::Signature.capabilities_name(),
            Some("requiresSignature")
        );
        assert_eq!(
            ShipmentOption::LargeFormat.capabilities_name(),
            Some("oversizedPackage")
        );
        assert_eq!(ShipmentOption::Tracked.capabilities_name(), None);
    }
}
