//! Return shipments.

use reqwest::Method;
use serde::Serializer;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::client::Client;
use crate::country::CountryCode;
use crate::enums::Carrier;
use crate::error::Result;
use crate::http::{Context, Host, Operation, Request, Response};
use crate::serde_util::{opt_string_or_int, ser_opt_int_bool};
use crate::shipments::{
    BoxNumber, CreatedShipment, CustomsDeclaration, EmailAddress, IncotermsRule,
    PhysicalProperties, Region,
};
use crate::types::{
    AccountId, ContractId, Insurance, LabelDescription, Money, NonEmpty, ReferenceIdentifier,
    ShipmentId, ShopId,
};

const RELATED_CONTENT_TYPE: &str = "application/vnd.return_shipment+json";
const UNRELATED_CONTENT_TYPE: &str = "application/vnd.unrelated_return_shipment+json";
const ACCEPT: &str = "application/json;charset=utf-8;version=1.2,application/json,application/vnd.shipment_label+json;q=0.9,application/problem+json;q=0.9,application/*;q=0.8";
const MAX_CONTRIBUTION_CENTS: u64 = 100_000;

int_enum! {
    /// A carrier that handles return shipments.
    pub enum ReturnCarrier(u8) {
        /// PostNL.
        PostNl = 1,
        /// bpost.
        Bpost = 2,
        /// DPD.
        Dpd = 4,
        /// DHL For You.
        DhlForYou = 9,
        /// DHL Parcel Connect.
        DhlParcelConnect = 10,
        /// DHL Europlus.
        DhlEuroplus = 11,
        /// GLS.
        Gls = 14,
        /// BRT.
        Brt = 15,
        /// InPost.
        InPost = 17,
        /// Poste Italiane.
        PosteItaliane = 18,
        /// Spring.
        Spring = 19,
    }
}

impl From<ReturnCarrier> for Carrier {
    fn from(carrier: ReturnCarrier) -> Self {
        Carrier::from_value(carrier.value()).expect("every return carrier is a carrier")
    }
}

int_enum! {
    /// A package type that can be returned.
    pub enum ReturnPackageType(u8) {
        /// A package.
        Package = 1,
        /// A mailbox package.
        Mailbox = 2,
        /// A small package.
        SmallPackage = 6,
    }
}

string_enum! {
    /// A yes/no option of a return shipment.
    pub enum ReturnOption {
        /// The recipient has to be 18 or older and sign.
        AgeCheck = "age_check",
        /// The shipment is delivered cooled.
        CooledDelivery = "cooled_delivery",
        /// The shipment is delivered at a postal point.
        DropOffAtPostalPoint = "drop_off_at_postal_point",
        /// Extra assurance.
        ExtraAssurance = "extra_assurance",
        /// The sender is hidden on the label.
        HideSender = "hide_sender",
        /// The shipment is large format.
        LargeFormat = "large_format",
        /// Only the recipient may receive the shipment.
        OnlyRecipient = "only_recipient",
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
    }
}

/// The options of a return shipment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReturnOptions {
    package_type: Option<ReturnPackageType>,
    flags: std::collections::BTreeMap<ReturnOption, bool>,
    printerless_return: Option<bool>,
    insurance: Option<Insurance>,
    contribution: Option<Money>,
    label_description: Option<LabelDescription>,
    incoterms: Option<IncotermsRule>,
}

impl ReturnOptions {
    /// Options that change nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the package type.
    pub fn package_type(mut self, value: ReturnPackageType) -> Self {
        self.package_type = Some(value);
        self
    }

    /// Switches an option on or off.
    pub fn flag(mut self, option: ReturnOption, enabled: bool) -> Self {
        self.flags.insert(option, enabled);
        self
    }

    /// Whether the consumer prints the label at a drop-off point instead of at home.
    pub fn printerless_return(mut self, value: bool) -> Self {
        self.printerless_return = Some(value);
        self
    }

    /// Sets the insured amount.
    pub fn insurance(mut self, value: Insurance) -> Self {
        self.insurance = Some(value);
        self
    }

    /// Sets the contribution of the consumer, at most 1000 euros.
    pub fn contribution(mut self, value: Money) -> Result<Self, crate::error::ValidationError> {
        if value.amount().get() > MAX_CONTRIBUTION_CENTS {
            return Err(crate::error::ValidationError::new(
                "contribution",
                format!(
                    "{} cents is more than {MAX_CONTRIBUTION_CENTS}",
                    value.amount().get()
                ),
            ));
        }
        self.contribution = Some(value);
        Ok(self)
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
}

impl Serialize for ReturnOptions {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Incoterms {
            rule: IncotermsRule,
        }

        let mut map = serializer.serialize_map(None)?;
        if let Some(value) = &self.package_type {
            map.serialize_entry("package_type", value)?;
        }
        for (option, enabled) in &self.flags {
            map.serialize_entry(option.as_str(), &u8::from(*enabled))?;
        }
        if let Some(value) = self.printerless_return {
            map.serialize_entry("printerless_return", &u8::from(value))?;
        }
        if let Some(value) = &self.insurance {
            map.serialize_entry("insurance", value)?;
        }
        if let Some(value) = &self.contribution {
            map.serialize_entry("contribution", value)?;
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

/// The address a return shipment is sent to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReturnSender {
    cc: CountryCode,
    city: String,
    street: String,
    person: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    street_additional_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    number_suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box_number: Option<BoxNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
    #[serde(skip_serializing_if = "Option::is_none")]
    company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<EmailAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<String>,
}

impl ReturnSender {
    /// An address with the fields the API requires. None of them may be empty.
    pub fn new(
        cc: CountryCode,
        city: impl Into<String>,
        street: impl Into<String>,
        person: impl Into<String>,
    ) -> std::result::Result<Self, crate::error::ValidationError> {
        let (city, street, person) = (city.into(), street.into(), person.into());
        for (field, value) in [("city", &city), ("street", &street), ("person", &person)] {
            if value.is_empty() {
                return Err(crate::error::ValidationError::new(
                    field,
                    "must not be empty",
                ));
            }
        }
        Ok(Self {
            cc,
            city,
            street,
            person,
            postal_code: None,
            street_additional_info: None,
            number: None,
            number_suffix: None,
            box_number: None,
            region: None,
            company: None,
            email: None,
            phone: None,
        })
    }

    /// Sets the postal code.
    pub fn postal_code(mut self, value: impl Into<String>) -> Self {
        self.postal_code = Some(value.into());
        self
    }

    /// Sets additional street information.
    pub fn street_additional_info(mut self, value: impl Into<String>) -> Self {
        self.street_additional_info = Some(value.into());
        self
    }

    /// Sets the house number.
    pub fn number(mut self, value: impl Into<String>) -> Self {
        self.number = Some(value.into());
        self
    }

    /// Sets the house number suffix.
    pub fn number_suffix(mut self, value: impl Into<String>) -> Self {
        self.number_suffix = Some(value.into());
        self
    }

    /// Sets the box number.
    pub fn box_number(mut self, value: BoxNumber) -> Self {
        self.box_number = Some(value);
        self
    }

    /// Sets the region.
    pub fn region(mut self, value: Region) -> Self {
        self.region = Some(value);
        self
    }

    /// Sets the company.
    pub fn company(mut self, value: impl Into<String>) -> Self {
        self.company = Some(value.into());
        self
    }

    /// Sets the email address.
    pub fn email(mut self, value: EmailAddress) -> Self {
        self.email = Some(value);
        self
    }

    /// Sets the phone number.
    pub fn phone(mut self, value: impl Into<String>) -> Self {
        self.phone = Some(value.into());
        self
    }
}

/// A return shipment for an existing shipment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelatedReturn {
    parent: ShipmentId,
    carrier: ReturnCarrier,
    email: EmailAddress,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<AccountId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shop_id: Option<ShopId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract_id: Option<ContractId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference_identifier: Option<ReferenceIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sender: Option<ReturnSender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    physical_properties: Option<PhysicalProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    customs_declaration: Option<CustomsDeclaration>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "save_recipient_address"
    )]
    save_recipient_address: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ReturnOptions>,
}

fn save_recipient_address<S: Serializer>(
    value: &Option<bool>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Settings {
        #[serde(serialize_with = "ser_opt_int_bool")]
        save_recipient_address: Option<bool>,
    }
    Settings {
        save_recipient_address: *value,
    }
    .serialize(serializer)
}

impl RelatedReturn {
    /// A return for `parent`, with the consumer's name and email address.
    ///
    /// The consumer receives the label by email when the return is created with
    /// `send_mail`.
    pub fn new(
        parent: ShipmentId,
        carrier: ReturnCarrier,
        name: impl Into<String>,
        email: EmailAddress,
    ) -> std::result::Result<Self, crate::error::ValidationError> {
        let name = name.into();
        if name.is_empty() {
            return Err(crate::error::ValidationError::new(
                "name",
                "must not be empty",
            ));
        }
        Ok(Self {
            parent,
            carrier,
            email,
            name,
            account_id: None,
            shop_id: None,
            contract_id: None,
            reference_identifier: None,
            sender: None,
            physical_properties: None,
            customs_declaration: None,
            save_recipient_address: None,
            options: None,
        })
    }

    /// Sets the account.
    pub fn account_id(mut self, value: AccountId) -> Self {
        self.account_id = Some(value);
        self
    }

    /// Sets the shop.
    pub fn shop_id(mut self, value: ShopId) -> Self {
        self.shop_id = Some(value);
        self
    }

    /// Sets the carrier contract.
    pub fn contract_id(mut self, value: ContractId) -> Self {
        self.contract_id = Some(value);
        self
    }

    /// Sets your own reference.
    pub fn reference_identifier(mut self, value: ReferenceIdentifier) -> Self {
        self.reference_identifier = Some(value);
        self
    }

    /// Sets the address the return is sent to.
    pub fn sender(mut self, value: ReturnSender) -> Self {
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

    /// Whether the recipient is saved in the address book.
    pub fn save_recipient_address(mut self, value: bool) -> Self {
        self.save_recipient_address = Some(value);
        self
    }

    /// Sets the options.
    pub fn options(mut self, value: ReturnOptions) -> Self {
        self.options = Some(value);
        self
    }
}

/// A return shipment that is not linked to an earlier shipment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnrelatedReturn {
    carrier: ReturnCarrier,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<AccountId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shop_id: Option<ShopId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<EmailAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<CountryCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sender: Option<ReturnSender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    physical_properties: Option<PhysicalProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    customs_declaration: Option<CustomsDeclaration>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "save_recipient_address"
    )]
    save_recipient_address: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ReturnOptions>,
}

impl UnrelatedReturn {
    /// A return with the only field the API requires.
    pub fn new(carrier: ReturnCarrier) -> Self {
        Self {
            carrier,
            account_id: None,
            shop_id: None,
            email: None,
            name: None,
            cc: None,
            sender: None,
            physical_properties: None,
            customs_declaration: None,
            save_recipient_address: None,
            options: None,
        }
    }

    /// Sets the account.
    pub fn account_id(mut self, value: AccountId) -> Self {
        self.account_id = Some(value);
        self
    }

    /// Sets the shop.
    pub fn shop_id(mut self, value: ShopId) -> Self {
        self.shop_id = Some(value);
        self
    }

    /// Sets the email address of the consumer.
    pub fn email(mut self, value: EmailAddress) -> Self {
        self.email = Some(value);
        self
    }

    /// Sets the name of the consumer.
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Sets the country.
    pub fn country(mut self, value: CountryCode) -> Self {
        self.cc = Some(value);
        self
    }

    /// Sets the address the return is sent to.
    pub fn sender(mut self, value: ReturnSender) -> Self {
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

    /// Whether the recipient is saved in the address book.
    pub fn save_recipient_address(mut self, value: bool) -> Self {
        self.save_recipient_address = Some(value);
        self
    }

    /// Sets the options.
    pub fn options(mut self, value: ReturnOptions) -> Self {
        self.options = Some(value);
        self
    }
}

#[derive(Deserialize)]
struct Envelope {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    #[serde(alias = "ids")]
    shipments: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    id: ShipmentId,
    #[serde(default, deserialize_with = "opt_string_or_int")]
    reference_identifier: Option<String>,
}

fn parse(response: Response) -> Result<Vec<CreatedShipment>> {
    let envelope: Envelope = response.ensure_success()?.json()?;
    Ok(envelope
        .data
        .shipments
        .into_iter()
        .map(|item| CreatedShipment {
            id: item.id,
            reference: item.reference_identifier,
        })
        .collect())
}

pub(crate) struct CreateRelated {
    body: serde_json::Value,
    send_mail: bool,
}

impl CreateRelated {
    pub fn new(returns: &NonEmpty<RelatedReturn>, send_mail: bool) -> Result<Self> {
        Ok(Self {
            body: json!({"data": {"return_shipments": serde_json::to_value(returns).map_err(|source| {
                crate::error::Error::Decode { source, body: String::new() }
            })?}}),
            send_mail,
        })
    }
}

impl Operation for CreateRelated {
    type Output = Vec<CreatedShipment>;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Request::new(Method::POST, Host::Core, vec!["shipments".into()], ACCEPT)
            .query("send_return_mail", if self.send_mail { "1" } else { "0" })
            .json_body(RELATED_CONTENT_TYPE, &self.body)
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        parse(response)
    }
}

pub(crate) struct CreateUnrelated {
    body: serde_json::Value,
}

impl CreateUnrelated {
    pub fn new(returns: &NonEmpty<UnrelatedReturn>) -> Result<Self> {
        Ok(Self {
            body: json!({"data": {"return_shipments": serde_json::to_value(returns).map_err(|source| {
                crate::error::Error::Decode { source, body: String::new() }
            })?}}),
        })
    }
}

impl Operation for CreateUnrelated {
    type Output = Vec<CreatedShipment>;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Request::new(Method::POST, Host::Core, vec!["shipments".into()], ACCEPT)
            .json_body(UNRELATED_CONTENT_TYPE, &self.body)
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        parse(response)
    }
}

macro_rules! returns_methods {
    ($($async:ident)?; $($await:ident)?) => {
        /// Creates return shipments for existing shipments.
        ///
        /// With `send_mail` MyParcel emails the label to the consumer. Leave it off while
        /// testing.
        pub $($async)? fn create_related(
            &self,
            returns: &$crate::types::NonEmpty<$crate::returns::RelatedReturn>,
            send_mail: bool,
        ) -> $crate::error::Result<Vec<$crate::shipments::CreatedShipment>> {
            self.client.execute($crate::returns::CreateRelated::new(returns, send_mail)?) $(.$await)?
        }

        /// Creates return shipments that are not linked to an earlier shipment.
        pub $($async)? fn create_unrelated(
            &self,
            returns: &$crate::types::NonEmpty<$crate::returns::UnrelatedReturn>,
        ) -> $crate::error::Result<Vec<$crate::shipments::CreatedShipment>> {
            self.client.execute($crate::returns::CreateUnrelated::new(returns)?) $(.$await)?
        }
    };
}

#[cfg(feature = "blocking")]
pub(crate) use returns_methods;

/// Operations on return shipments.
#[derive(Debug, Clone, Copy)]
pub struct Returns<'a> {
    pub(crate) client: &'a Client,
}

impl Returns<'_> {
    returns_methods!(async; await);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn related_return_serializes_the_required_fields() {
        let item = RelatedReturn::new(
            ShipmentId(12345),
            ReturnCarrier::PostNl,
            "John Doe",
            EmailAddress::new("john@example.com").unwrap(),
        )
        .unwrap()
        .options(
            ReturnOptions::new()
                .flag(ReturnOption::Signature, true)
                .printerless_return(true),
        );
        assert_eq!(
            serde_json::to_string(&item).unwrap(),
            r#"{"parent":12345,"carrier":1,"email":"john@example.com","name":"John Doe","options":{"signature":1,"printerless_return":1}}"#
        );
    }

    #[test]
    fn contribution_is_capped() {
        let over = Money::eur(crate::types::Cents::new(100_001));
        assert!(ReturnOptions::new().contribution(over).is_err());
    }

    #[test]
    fn every_return_carrier_is_a_carrier() {
        for carrier in ReturnCarrier::ALL {
            let _ = Carrier::from(*carrier);
        }
    }
}
