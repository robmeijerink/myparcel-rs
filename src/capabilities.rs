//! What a carrier can do for a destination, and what an account's contracts offer.

use std::collections::{BTreeMap, BTreeSet};

use reqwest::Method;
use serde::Serializer;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::client::Client;
use crate::country::CountryCode;
use crate::enums::{
    CarrierV2, DeliveryType, DeliveryTypeV2, Direction, LocationType, PackageType, PackageTypeV2,
    TransactionType,
};
use crate::error::{Error, Result, ValidationError};
use crate::http::{Context, Host, Operation, Request, Response};
use crate::reported::Reported;
use crate::shipments::response::Price;
use crate::shipments::{NewShipment, ShipmentOption};
use crate::types::{Cents, ContractId, ShopId};

const CONTENT_TYPE: &str = "application/json;version=2.0";
const ACCEPT: &str = "application/json;charset=utf-8;version=2.0,application/json;charset=utf-8,application/json,application/*;q=0.9";

string_enum! {
    /// A shipment option as the capabilities API names it.
    pub enum CapabilityOption {
        /// Additional insurance.
        AdditionalInsurance = "additionalInsurance",
        /// Deliver at a postal point.
        DeliverAtPostalPoint = "deliverAtPostalPoint",
        /// Fresh food.
        FreshFood = "freshFood",
        /// Frozen.
        Frozen = "frozen",
        /// Hide the sender.
        HideSender = "hideSender",
        /// Insurance.
        Insurance = "insurance",
        /// No tracking.
        NoTracking = "noTracking",
        /// Oversized package.
        OversizedPackage = "oversizedPackage",
        /// Print the return label at the drop-off point.
        PrintReturnLabelAtDropOff = "printReturnLabelAtDropOff",
        /// Priority delivery.
        PriorityDelivery = "priorityDelivery",
        /// Only the recipient may receive the shipment.
        RecipientOnlyDelivery = "recipientOnlyDelivery",
        /// Return on the first failed delivery.
        ReturnOnFirstFailedDelivery = "returnOnFirstFailedDelivery",
        /// Age verification.
        RequiresAgeVerification = "requiresAgeVerification",
        /// Cash on delivery.
        RequiresCashOnDelivery = "requiresCashOnDelivery",
        /// Receipt code.
        RequiresReceiptCode = "requiresReceiptCode",
        /// Signature.
        RequiresSignature = "requiresSignature",
        /// Same-day delivery.
        SameDayDelivery = "sameDayDelivery",
        /// Saturday delivery.
        SaturdayDelivery = "saturdayDelivery",
        /// Scheduled collection.
        ScheduledCollection = "scheduledCollection",
    }
}

impl ShipmentOption {
    /// The matching option of the capabilities API, if it has one.
    pub fn capability_option(self) -> Option<CapabilityOption> {
        self.capabilities_name()
            .and_then(CapabilityOption::from_name)
    }
}

/// How a shipment is insured in a capabilities request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsuranceRequest {
    /// The carrier's default coverage.
    Default,
    /// An insured amount. Zero means no insurance.
    Amount(Cents),
}

string_enum! {
    /// The unit of a length.
    pub enum LengthUnit {
        /// Centimetres.
        Cm = "cm",
        /// Millimetres.
        Mm = "mm",
    }
}

string_enum! {
    /// The unit of a weight.
    pub enum WeightUnit {
        /// Grams.
        G = "g",
        /// Kilograms.
        Kg = "kg",
    }
}

/// A length from 0 to 10 000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Length {
    value: u16,
    unit: LengthUnit,
}

impl Length {
    /// Validates a length.
    pub fn new(value: u16, unit: LengthUnit) -> std::result::Result<Self, ValidationError> {
        if value > 10_000 {
            return Err(ValidationError::new(
                "length",
                format!("{value} is more than 10000"),
            ));
        }
        Ok(Self { value, unit })
    }
}

/// A weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Weight {
    value: u32,
    unit: WeightUnit,
}

impl Weight {
    /// A weight in a unit.
    pub fn new(value: u32, unit: WeightUnit) -> Self {
        Self { value, unit }
    }
}

/// The size and weight of the shipment a capabilities request is about.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CapabilityProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight: Option<Weight>,
}

impl CapabilityProperties {
    /// Properties without any value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the height.
    pub fn height(mut self, value: Length) -> Self {
        self.height = Some(value);
        self
    }

    /// Sets the width.
    pub fn width(mut self, value: Length) -> Self {
        self.width = Some(value);
        self
    }

    /// Sets the length.
    pub fn length(mut self, value: Length) -> Self {
        self.length = Some(value);
        self
    }

    /// Sets the weight.
    pub fn weight(mut self, value: Weight) -> Self {
        self.weight = Some(value);
        self
    }
}

bounded_string! {
    /// A postal code of at most 15 characters.
    CapabilityPostalCode, "postal code", 1, 15
}

/// One end of a shipment in a capabilities request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Party {
    country_code: CountryCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    postal_code: Option<CapabilityPostalCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_business: Option<bool>,
}

impl Party {
    /// A party in a country.
    pub fn new(country_code: CountryCode) -> Self {
        Self {
            country_code,
            postal_code: None,
            is_business: None,
        }
    }

    /// Sets the postal code.
    pub fn postal_code(mut self, value: CapabilityPostalCode) -> Self {
        self.postal_code = Some(value);
        self
    }

    /// Sets whether the party is a business.
    pub fn is_business(mut self, value: bool) -> Self {
        self.is_business = Some(value);
        self
    }
}

/// A question to the capabilities API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitiesRequest {
    shop_id: Option<ShopId>,
    contract_id: Option<ContractId>,
    recipient: Party,
    sender: Option<Party>,
    carrier: Option<CarrierV2>,
    package_type: Option<PackageTypeV2>,
    delivery_type: Option<DeliveryTypeV2>,
    direction: Option<Direction>,
    physical_properties: Option<CapabilityProperties>,
    options: BTreeSet<CapabilityOption>,
    insurance: Option<InsuranceRequest>,
    pickup_location: Option<LocationType>,
}

impl CapabilitiesRequest {
    /// A question about shipments to a country.
    pub fn for_country(country: CountryCode) -> Self {
        Self::for_recipient(Party::new(country))
    }

    /// A question about shipments to a recipient.
    pub fn for_recipient(recipient: Party) -> Self {
        Self {
            shop_id: None,
            contract_id: None,
            recipient,
            sender: None,
            carrier: None,
            package_type: None,
            delivery_type: None,
            direction: None,
            physical_properties: None,
            options: BTreeSet::new(),
            insurance: None,
            pickup_location: None,
        }
    }

    /// A question about a shipment you are about to create.
    ///
    /// Takes the recipient's country and the weight in grams. The size is left out because the
    /// unit of a shipment's dimensions is not documented.
    pub fn from_shipment(shipment: &NewShipment) -> Self {
        let request = Self::for_country(shipment.recipient_country());
        match shipment.weight() {
            Some(weight) => request.physical_properties(
                CapabilityProperties::new().weight(Weight::new(weight.get(), WeightUnit::G)),
            ),
            None => request,
        }
    }

    /// Limits the question to a shop.
    pub fn shop_id(mut self, value: ShopId) -> Self {
        self.shop_id = Some(value);
        self
    }

    /// Limits the question to a contract.
    pub fn contract_id(mut self, value: ContractId) -> Self {
        self.contract_id = Some(value);
        self
    }

    /// Sets the sender.
    pub fn sender(mut self, value: Party) -> Self {
        self.sender = Some(value);
        self
    }

    /// Limits the question to a carrier.
    pub fn carrier(mut self, value: CarrierV2) -> Self {
        self.carrier = Some(value);
        self
    }

    /// Limits the question to a package type.
    pub fn package_type(mut self, value: PackageType) -> Self {
        self.package_type = Some(value.into());
        self
    }

    /// Limits the question to a delivery type.
    pub fn delivery_type(mut self, value: DeliveryType) -> Self {
        self.delivery_type = Some(value.into());
        self
    }

    /// Sets the direction of the shipment.
    pub fn direction(mut self, value: Direction) -> Self {
        self.direction = Some(value);
        self
    }

    /// Sets the size and weight.
    pub fn physical_properties(mut self, value: CapabilityProperties) -> Self {
        self.physical_properties = Some(value);
        self
    }

    /// Asks for an option to be available. Use [`insurance`](Self::insurance) for insurance.
    pub fn option(mut self, value: CapabilityOption) -> Self {
        self.options.insert(value);
        self
    }

    /// Asks for insurance.
    pub fn insurance(mut self, value: InsuranceRequest) -> Self {
        self.insurance = Some(value);
        self
    }

    /// Limits the question to a kind of pick-up location.
    pub fn pickup_location(mut self, value: LocationType) -> Self {
        self.pickup_location = Some(value);
        self
    }
}

impl Serialize for CapabilitiesRequest {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        struct Options<'a>(&'a CapabilitiesRequest);

        impl Serialize for Options<'_> {
            fn serialize<S: Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                let mut map = serializer.serialize_map(None)?;
                for option in &self.0.options {
                    if *option != CapabilityOption::Insurance {
                        map.serialize_entry(option.as_str(), &json!({}))?;
                    }
                }
                let insured = self.0.insurance.is_some()
                    || self.0.options.contains(&CapabilityOption::Insurance);
                if insured {
                    match self.0.insurance {
                        Some(InsuranceRequest::Amount(amount)) => map.serialize_entry(
                            "insurance",
                            &json!({"currency": "EUR", "amount": amount.get()}),
                        )?,
                        _ => map.serialize_entry("insurance", &json!({}))?,
                    }
                }
                map.end()
            }
        }

        let mut map = serializer.serialize_map(None)?;
        if let Some(value) = &self.shop_id {
            map.serialize_entry("shopId", value)?;
        }
        if let Some(value) = &self.contract_id {
            map.serialize_entry("contractId", value)?;
        }
        map.serialize_entry("recipient", &self.recipient)?;
        if let Some(value) = &self.sender {
            map.serialize_entry("sender", value)?;
        }
        if let Some(value) = &self.carrier {
            map.serialize_entry("carrier", value)?;
        }
        if let Some(value) = &self.package_type {
            map.serialize_entry("packageType", value)?;
        }
        if let Some(value) = &self.delivery_type {
            map.serialize_entry("deliveryType", value)?;
        }
        if let Some(value) = &self.direction {
            map.serialize_entry("direction", value)?;
        }
        if let Some(value) = &self.physical_properties {
            map.serialize_entry("physicalProperties", value)?;
        }
        if !self.options.is_empty() || self.insurance.is_some() {
            map.serialize_entry("options", &Options(self))?;
        }
        if let Some(value) = &self.pickup_location {
            map.serialize_entry("pickup", &json!({"location": {"type": value}}))?;
        }
        map.end()
    }
}

/// Whether an option can be used and how it relates to others.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionInfo {
    /// Whether the option is on unless switched off.
    pub is_selected_by_default: bool,
    /// Whether the option has to be used.
    pub is_required: bool,
    /// Options that have to be used together with this one.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Options that cannot be used together with this one.
    #[serde(default)]
    pub excludes: Vec<String>,
}

/// The insurance limits of a carrier.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsuranceInfo {
    /// Whether insurance is on unless switched off.
    pub is_selected_by_default: bool,
    /// Whether insurance has to be used.
    pub is_required: bool,
    /// Options that have to be used together with insurance.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Options that cannot be used together with insurance.
    #[serde(default)]
    pub excludes: Vec<String>,
    /// The insured amount.
    pub insured_amount: Option<AmountRange>,
    /// The default amount.
    pub default: Option<Price>,
    /// The smallest amount.
    pub min: Option<Price>,
    /// The largest amount.
    pub max: Option<Price>,
}

/// A range of amounts.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AmountRange {
    /// The default amount.
    pub default: Option<Price>,
    /// The smallest amount.
    pub min: Option<Price>,
    /// The largest amount.
    pub max: Option<Price>,
}

/// A value with its unit.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Measure {
    /// The value.
    pub value: f64,
    /// The unit, such as `g` or `cm`.
    pub unit: String,
}

/// The limits of a physical property.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PropertyLimits {
    /// The smallest value.
    pub min: Measure,
    /// The largest value.
    pub max: Measure,
    /// How the value is calculated.
    pub calculation: String,
}

/// The contract a capability comes from.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Contract {
    /// The contract id.
    pub id: ContractId,
    /// Whether this is the main contract or a custom one.
    #[serde(rename = "type")]
    pub kind: Reported<ContractType>,
}

string_enum! {
    /// The kind of a carrier contract.
    pub enum ContractType {
        /// The main contract.
        Main = "MAIN",
        /// A custom contract.
        Custom = "CUSTOM",
    }
}

/// The number of colli a shipment can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Collo {
    /// The largest number of colli.
    pub max: u32,
}

type OptionMap = BTreeMap<String, OptionInfo>;

fn split_options(
    mut raw: BTreeMap<String, Value>,
) -> std::result::Result<(OptionMap, Option<InsuranceInfo>), String> {
    let insurance = raw
        .remove("insurance")
        .map(|value| InsuranceInfo::deserialize(value).map_err(|error| error.to_string()))
        .transpose()?;
    let options = raw
        .into_iter()
        .map(|(name, value)| {
            OptionInfo::deserialize(value)
                .map(|info| (name, info))
                .map_err(|error| error.to_string())
        })
        .collect::<std::result::Result<_, _>>()?;
    Ok((options, insurance))
}

/// What one carrier offers for the asked destination.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "RawCapability")]
pub struct Capability {
    /// The carrier.
    pub carrier: Reported<CarrierV2>,
    /// The contract, when the answer is contract specific.
    pub contract: Option<Contract>,
    /// The package types.
    pub package_types: Vec<Reported<PackageTypeV2>>,
    /// The delivery types.
    pub delivery_types: Vec<Reported<DeliveryTypeV2>>,
    /// The transaction types.
    pub transaction_types: Vec<Reported<TransactionType>>,
    /// The number of colli.
    pub collo: Collo,
    /// The options, keyed by their name, without insurance.
    pub options: OptionMap,
    /// The insurance limits, when insurance is offered.
    pub insurance: Option<InsuranceInfo>,
    /// The limits of the physical properties, keyed by their name.
    pub physical_properties: BTreeMap<String, PropertyLimits>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCapability {
    carrier: Reported<CarrierV2>,
    contract: Option<Contract>,
    package_types: Vec<Reported<PackageTypeV2>>,
    delivery_types: Vec<Reported<DeliveryTypeV2>>,
    transaction_types: Vec<Reported<TransactionType>>,
    collo: Collo,
    options: BTreeMap<String, Value>,
    #[serde(default)]
    physical_properties: BTreeMap<String, PropertyLimits>,
}

impl TryFrom<RawCapability> for Capability {
    type Error = String;

    fn try_from(raw: RawCapability) -> std::result::Result<Self, Self::Error> {
        let (options, insurance) = split_options(raw.options)?;
        Ok(Self {
            carrier: raw.carrier,
            contract: raw.contract,
            package_types: raw.package_types,
            delivery_types: raw.delivery_types,
            transaction_types: raw.transaction_types,
            collo: raw.collo,
            options,
            insurance,
            physical_properties: raw.physical_properties,
        })
    }
}

impl Capability {
    /// The information about an option, if the carrier offers it.
    pub fn option(&self, option: CapabilityOption) -> Option<&OptionInfo> {
        self.options.get(option.as_str())
    }
}

/// The answer of the capabilities API.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CapabilitiesResponse {
    /// One entry per carrier and contract that matches.
    pub results: Vec<Capability>,
}

impl CapabilitiesResponse {
    /// The package types of all results, without duplicates.
    pub fn package_types(&self) -> Vec<&Reported<PackageTypeV2>> {
        unique(self.results.iter().flat_map(|c| &c.package_types))
    }

    /// The delivery types of all results, without duplicates.
    pub fn delivery_types(&self) -> Vec<&Reported<DeliveryTypeV2>> {
        unique(self.results.iter().flat_map(|c| &c.delivery_types))
    }

    /// The transaction types of all results, without duplicates.
    pub fn transaction_types(&self) -> Vec<&Reported<TransactionType>> {
        unique(self.results.iter().flat_map(|c| &c.transaction_types))
    }

    /// The names of the options of all results, without duplicates.
    pub fn shipment_options(&self) -> BTreeSet<&str> {
        self.results
            .iter()
            .flat_map(|c| c.options.keys().map(String::as_str))
            .collect()
    }

    /// The carrier, when all results are about the same one.
    pub fn carrier(&self) -> Option<&Reported<CarrierV2>> {
        let first = &self.results.first()?.carrier;
        self.results
            .iter()
            .all(|c| &c.carrier == first)
            .then_some(first)
    }

    /// The largest number of colli of all results.
    pub fn collo_max(&self) -> Option<u32> {
        self.results.iter().map(|c| c.collo.max).max()
    }
}

fn unique<'a, T: PartialEq>(items: impl Iterator<Item = &'a T>) -> Vec<&'a T> {
    let mut result: Vec<&T> = Vec::new();
    for item in items {
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
}

string_enum! {
    /// A carrier that has contract definitions.
    pub enum ContractCarrier {
        /// BRT.
        Brt = "BRT",
        /// DHL Europlus.
        DhlEuroplus = "DHL_EUROPLUS",
        /// DHL Parcel Connect.
        DhlParcelConnect = "DHL_PARCEL_CONNECT",
        /// DPD.
        Dpd = "DPD",
        /// DHL For You.
        DhlForYou = "DHL_FOR_YOU",
        /// InPost.
        InPost = "INPOST",
        /// PostNL.
        PostNl = "POSTNL",
    }
}

/// What a carrier offers across all contracts of the account.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "RawContractDefinition")]
pub struct ContractDefinition {
    /// The carrier.
    pub carrier: Reported<CarrierV2>,
    /// The package types.
    pub package_types: Vec<Reported<PackageTypeV2>>,
    /// The delivery types.
    pub delivery_types: Vec<Reported<DeliveryTypeV2>>,
    /// The transaction types.
    pub transaction_types: Vec<Reported<TransactionType>>,
    /// The number of colli.
    pub collo: Collo,
    /// The options, keyed by their name, without insurance.
    pub options: OptionMap,
    /// The insurance limits, when insurance is offered.
    pub insurance: Option<InsuranceInfo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawContractDefinition {
    carrier: Reported<CarrierV2>,
    package_types: Vec<Reported<PackageTypeV2>>,
    delivery_types: Vec<Reported<DeliveryTypeV2>>,
    transaction_types: Vec<Reported<TransactionType>>,
    collo: Collo,
    options: BTreeMap<String, Value>,
}

impl TryFrom<RawContractDefinition> for ContractDefinition {
    type Error = String;

    fn try_from(raw: RawContractDefinition) -> std::result::Result<Self, Self::Error> {
        let (options, insurance) = split_options(raw.options)?;
        Ok(Self {
            carrier: raw.carrier,
            package_types: raw.package_types,
            delivery_types: raw.delivery_types,
            transaction_types: raw.transaction_types,
            collo: raw.collo,
            options,
            insurance,
        })
    }
}

pub(crate) struct Get {
    body: Value,
}

impl Get {
    pub fn new(request: &CapabilitiesRequest) -> Result<Self> {
        let body = serde_json::to_value(request).map_err(|source| Error::Decode {
            source,
            body: String::new(),
        })?;
        Ok(Self { body })
    }
}

impl Operation for Get {
    type Output = CapabilitiesResponse;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Request::new(
            Method::POST,
            Host::Core,
            vec!["shipments".into(), "capabilities".into()],
            ACCEPT,
        )
        .json_body(CONTENT_TYPE, &self.body)
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        response.ensure_success()?.json()
    }
}

pub(crate) struct ContractDefinitions {
    body: Value,
}

impl ContractDefinitions {
    pub fn new(carrier: Option<ContractCarrier>) -> Self {
        let body = match carrier {
            Some(carrier) => json!({"carrier": carrier}),
            None => json!({}),
        };
        Self { body }
    }
}

impl Operation for ContractDefinitions {
    type Output = Vec<ContractDefinition>;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Request::new(
            Method::POST,
            Host::Core,
            vec![
                "shipments".into(),
                "capabilities".into(),
                "contract-definitions".into(),
            ],
            ACCEPT,
        )
        .json_body(CONTENT_TYPE, &self.body)
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        #[derive(Deserialize)]
        struct Envelope {
            items: Vec<ContractDefinition>,
        }

        Ok(response.ensure_success()?.json::<Envelope>()?.items)
    }
}

macro_rules! capabilities_methods {
    ($($async:ident)?; $($await:ident)?) => {
        /// Asks what carriers can do for a destination.
        pub $($async)? fn get(
            &self,
            request: &$crate::capabilities::CapabilitiesRequest,
        ) -> $crate::error::Result<$crate::capabilities::CapabilitiesResponse> {
            self.client.execute($crate::capabilities::Get::new(request)?) $(.$await)?
        }

        /// Lists what each carrier offers across all contracts of the account.
        ///
        /// This is the superset over all contracts, not a list of contracts. Use
        /// [`get`](Self::get) for answers about a destination.
        pub $($async)? fn contract_definitions(
            &self,
            carrier: Option<$crate::capabilities::ContractCarrier>,
        ) -> $crate::error::Result<Vec<$crate::capabilities::ContractDefinition>> {
            self.client.execute($crate::capabilities::ContractDefinitions::new(carrier)) $(.$await)?
        }
    };
}

#[cfg(feature = "blocking")]
pub(crate) use capabilities_methods;

/// Operations on carrier capabilities.
#[derive(Debug, Clone, Copy)]
pub struct Capabilities<'a> {
    pub(crate) client: &'a Client,
}

impl Capabilities<'_> {
    capabilities_methods!(async; await);
}
