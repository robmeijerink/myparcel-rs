//! Orders pushed to MyParcel through Connect.

use std::collections::BTreeSet;

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use time::OffsetDateTime;

use crate::country::CountryCode;
use crate::enums::{CarrierV2, PackageTypeV2};
use crate::error::ValidationError;
use crate::types::NonEmpty;

bounded_string! {
    /// A city of 1 to 100 characters.
    OrderCity, "city", 1, 100
}
bounded_string! {
    /// A street of 1 to 100 characters.
    OrderStreet, "street", 1, 100
}
bounded_string! {
    /// Additional address information of 1 to 100 characters.
    AdditionalInformation, "additional information", 1, 100
}
bounded_string! {
    /// A box number of 1 to 20 characters.
    OrderBoxNumber, "box number", 1, 20
}
bounded_string! {
    /// A house number of 1 to 25 characters.
    OrderHouseNumber, "house number", 1, 25
}
bounded_string! {
    /// A house number suffix of 1 to 25 characters.
    OrderHouseNumberSuffix, "house number suffix", 1, 25
}
bounded_string! {
    /// A postal code of 1 to 15 characters.
    OrderPostalCode, "postal code", 1, 15
}
bounded_string! {
    /// A region or state of 1 to 100 characters.
    OrderRegion, "region", 1, 100
}
bounded_string! {
    /// An email address of 1 to 320 characters.
    ContactEmail, "email address", 1, 320
}
bounded_string! {
    /// A phone number of 1 to 50 characters.
    ContactPhone, "phone number", 1, 50
}
bounded_string! {
    /// A company name of 1 to 100 characters.
    ContactCompany, "company", 1, 100
}
bounded_string! {
    /// A person's name of 1 to 200 characters.
    ContactName, "name", 1, 200
}
bounded_string! {
    /// A locale of 2 or 5 characters.
    Locale, "locale", 2, 5,
    |value| (value.chars().count() == 3 || value.chars().count() == 4).then_some("must have 2 or 5 characters")
}
bounded_string! {
    /// An identifier in your system, 1 to 50 characters.
    SourceId, "source id", 1, 50
}
bounded_string! {
    /// A customer reference of 1 to 100 characters.
    CustomerReference, "customer reference", 1, 100
}
bounded_string! {
    /// A barcode of 6 to 24 characters.
    ProductBarcode, "barcode", 6, 24
}
bounded_string! {
    /// An EAN of 1 to 13 characters.
    Ean, "ean", 1, 13
}
bounded_string! {
    /// A harmonised system code of 6 to 10 characters.
    HsCode, "hs code", 6, 10
}
bounded_string! {
    /// A description of 1 to 2500 characters.
    DescriptionText, "description", 1, 2500
}
bounded_string! {
    /// A two letter language code.
    LanguageCode, "language code", 2, 2
}
bounded_string! {
    /// Text printed on the label, 1 to 45 characters.
    CustomLabelText, "custom label text", 1, 45
}
bounded_string! {
    /// Additional shipping context of 1 to 256 characters.
    AdditionalContext, "additional context", 1, 256
}
bounded_string! {
    /// The name of a pick-up location, 1 to 200 characters.
    PickupName, "pick-up location name", 1, 200
}
bounded_string! {
    /// The code of a pick-up location, 1 to 64 characters.
    PickupCode, "pick-up location code", 1, 64
}
bounded_string! {
    /// An ISO 4217 currency code of 3 characters.
    CurrencyCode, "currency", 3, 3
}

string_enum! {
    /// The status of an order.
    pub enum OrderStatus {
        /// The order is open.
        Open = "OPEN",
        /// The order is canceled.
        Canceled = "CANCELED",
    }
}

string_enum! {
    /// How an order is delivered.
    pub enum OrderDeliveryType {
        /// Early morning delivery.
        EarlyMorning = "EARLY_MORNING_DELIVERY",
        /// Evening delivery.
        Evening = "EVENING_DELIVERY",
        /// Express delivery.
        Express = "EXPRESS_DELIVERY",
        /// Morning delivery.
        Morning = "MORNING_DELIVERY",
        /// Express delivery to a pick-up location.
        PickupExpress = "PICKUP_EXPRESS_DELIVERY",
        /// Delivery to a pick-up location.
        Pickup = "PICKUP_DELIVERY",
        /// Same-day delivery.
        SameDay = "SAME_DAY_DELIVERY",
        /// Standard delivery.
        Standard = "STANDARD_DELIVERY",
    }
}

string_enum! {
    /// A yes/no option of an order's shipment.
    pub enum OrderOption {
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
        /// Age verification.
        RequiresAgeVerification = "requiresAgeVerification",
        /// Cash on delivery.
        RequiresCashOnDelivery = "requiresCashOnDelivery",
        /// Receipt code.
        RequiresReceiptCode = "requiresReceiptCode",
        /// Signature.
        RequiresSignature = "requiresSignature",
        /// Return on the first failed delivery.
        ReturnOnFirstFailedDelivery = "returnOnFirstFailedDelivery",
        /// Same-day delivery.
        SameDayDelivery = "sameDayDelivery",
        /// Saturday delivery.
        SaturdayDelivery = "saturdayDelivery",
        /// Scheduled collection.
        ScheduledCollection = "scheduledCollection",
    }
}

string_enum! {
    /// The kind of bpost pick-up location.
    pub enum BpostPickupType {
        /// A click and collect point.
        ClickCollect = "CLICK_COLLECT",
        /// A parcel locker.
        ParcelLocker = "PARCEL_LOCKER",
        /// A parcel point.
        ParcelPoint = "PARCEL_POINT",
        /// A post office.
        PostOffice = "POST_OFFICE",
        /// A post point.
        PostPoint = "POST_POINT",
    }
}

string_enum! {
    /// The PostNL retail network of a pick-up location.
    pub enum RetailNetwork {
        /// The Dutch network.
        Netherlands = "PNPNL-01",
        /// The Belgian network.
        Belgium = "PNPBE-01",
    }
}

/// An amount of money in millionths of the currency: 24.95 euros is 24 950 000 micros.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrderMoney {
    currency: CurrencyCode,
    micros: u64,
}

impl OrderMoney {
    /// The largest amount of micros the API accepts.
    pub const MAX_MICROS: u64 = 9_007_199_254_740_991;

    /// Validates an amount.
    pub fn new(currency: CurrencyCode, micros: u64) -> Result<Self, ValidationError> {
        if micros > Self::MAX_MICROS {
            return Err(ValidationError::new(
                "amount",
                format!("{micros} micros is too large"),
            ));
        }
        Ok(Self { currency, micros })
    }
}

/// A price with the tax rate in percent, 0 to 100.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesPrice {
    currency: CurrencyCode,
    micros: u64,
    tax_rate: u8,
}

impl SalesPrice {
    /// Validates a price.
    pub fn new(money: OrderMoney, tax_rate: u8) -> Result<Self, ValidationError> {
        if tax_rate > 100 {
            return Err(ValidationError::new(
                "tax rate",
                format!("{tax_rate} is more than 100"),
            ));
        }
        Ok(Self {
            currency: money.currency,
            micros: money.micros,
            tax_rate,
        })
    }
}

/// An address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderAddress {
    city: OrderCity,
    country_code: CountryCode,
    street: OrderStreet,
    #[serde(skip_serializing_if = "Option::is_none")]
    additional_information: Option<AdditionalInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box_number: Option<OrderBoxNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    house_number: Option<OrderHouseNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    house_number_suffix: Option<OrderHouseNumberSuffix>,
    #[serde(skip_serializing_if = "Option::is_none")]
    postal_code: Option<OrderPostalCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<OrderRegion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<OrderRegion>,
}

impl OrderAddress {
    /// An address with the fields the API requires.
    pub fn new(city: OrderCity, country_code: CountryCode, street: OrderStreet) -> Self {
        Self {
            city,
            country_code,
            street,
            additional_information: None,
            box_number: None,
            house_number: None,
            house_number_suffix: None,
            postal_code: None,
            region: None,
            state: None,
        }
    }

    /// Sets additional information.
    pub fn additional_information(mut self, value: AdditionalInformation) -> Self {
        self.additional_information = Some(value);
        self
    }

    /// Sets the box number.
    pub fn box_number(mut self, value: OrderBoxNumber) -> Self {
        self.box_number = Some(value);
        self
    }

    /// Sets the house number.
    pub fn house_number(mut self, value: OrderHouseNumber) -> Self {
        self.house_number = Some(value);
        self
    }

    /// Sets the house number suffix.
    pub fn house_number_suffix(mut self, value: OrderHouseNumberSuffix) -> Self {
        self.house_number_suffix = Some(value);
        self
    }

    /// Sets the postal code.
    pub fn postal_code(mut self, value: OrderPostalCode) -> Self {
        self.postal_code = Some(value);
        self
    }

    /// Sets the region.
    pub fn region(mut self, value: OrderRegion) -> Self {
        self.region = Some(value);
        self
    }

    /// Sets the state.
    pub fn state(mut self, value: OrderRegion) -> Self {
        self.state = Some(value);
        self
    }
}

/// Who a contact is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContactKind {
    /// A business, with an optional person to address.
    Business {
        /// The company.
        company: ContactCompany,
        /// The person to address.
        attention: Option<ContactCompany>,
    },
    /// A private person.
    Private {
        /// The name.
        name: ContactName,
    },
}

/// A contact of the billing details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    kind: ContactKind,
    email_address: Option<ContactEmail>,
    locale: Option<Locale>,
    phone_number: Option<ContactPhone>,
}

impl Contact {
    /// A contact of a kind.
    pub fn new(kind: ContactKind) -> Self {
        Self {
            kind,
            email_address: None,
            locale: None,
            phone_number: None,
        }
    }

    /// Sets the email address.
    pub fn email_address(mut self, value: ContactEmail) -> Self {
        self.email_address = Some(value);
        self
    }

    /// Sets the locale.
    pub fn locale(mut self, value: Locale) -> Self {
        self.locale = Some(value);
        self
    }

    /// Sets the phone number.
    pub fn phone_number(mut self, value: ContactPhone) -> Self {
        self.phone_number = Some(value);
        self
    }
}

fn serialize_contact<S: Serializer>(
    contact: &Contact,
    emails: Option<&NonEmpty<ContactEmail, 5>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut map = serializer.serialize_map(None)?;
    match &contact.kind {
        ContactKind::Business { company, attention } => {
            map.serialize_entry("type", "BUSINESS")?;
            map.serialize_entry("company", company)?;
            if let Some(attention) = attention {
                map.serialize_entry("attention", attention)?;
            }
        }
        ContactKind::Private { name } => {
            map.serialize_entry("type", "PRIVATE")?;
            map.serialize_entry("name", name)?;
        }
    }
    match emails {
        Some(emails) => map.serialize_entry("emailAddresses", emails)?,
        None => {
            if let Some(email) = &contact.email_address {
                map.serialize_entry("emailAddress", email)?;
            }
        }
    }
    if let Some(locale) = &contact.locale {
        map.serialize_entry("locale", locale)?;
    }
    if let Some(phone) = &contact.phone_number {
        map.serialize_entry("phoneNumber", phone)?;
    }
    map.end()
}

impl Serialize for Contact {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_contact(self, None, serializer)
    }
}

/// A contact that can have up to five email addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipientContact {
    contact: Contact,
    email_addresses: Option<NonEmpty<ContactEmail, 5>>,
}

impl RecipientContact {
    /// A contact of a kind.
    pub fn new(kind: ContactKind) -> Self {
        Self {
            contact: Contact::new(kind),
            email_addresses: None,
        }
    }

    /// Sets the email addresses.
    pub fn email_addresses(mut self, value: NonEmpty<ContactEmail, 5>) -> Self {
        self.email_addresses = Some(value);
        self
    }

    /// Sets the locale.
    pub fn locale(mut self, value: Locale) -> Self {
        self.contact = self.contact.locale(value);
        self
    }

    /// Sets the phone number.
    pub fn phone_number(mut self, value: ContactPhone) -> Self {
        self.contact = self.contact.phone_number(value);
        self
    }
}

impl Serialize for RecipientContact {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_contact(&self.contact, self.email_addresses.as_ref(), serializer)
    }
}

/// The billing details of an order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BillingDetails {
    /// The address.
    pub address: OrderAddress,
    /// The contact.
    pub contact: Contact,
}

/// The recipient of an order's shipment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrderRecipient {
    /// The address.
    pub address: OrderAddress,
    /// The contact.
    pub contact: RecipientContact,
}

/// A length with its unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Dimension {
    unit: DimensionUnit,
    value: u32,
}

string_enum! {
    /// The unit of a dimension.
    pub enum DimensionUnit {
        /// Millimetres.
        Mm = "mm",
        /// Centimetres.
        Cm = "cm",
        /// Metres.
        M = "m",
    }
}

string_enum! {
    /// The unit of a weight.
    pub enum OrderWeightUnit {
        /// Grams.
        G = "g",
        /// Kilograms.
        Kg = "kg",
    }
}

impl Dimension {
    /// Validates a dimension: 1 to 1 000 000.
    pub fn new(value: u32, unit: DimensionUnit) -> Result<Self, ValidationError> {
        if !(1..=1_000_000).contains(&value) {
            return Err(ValidationError::new(
                "dimension",
                format!("{value} is outside 1..=1000000"),
            ));
        }
        Ok(Self { unit, value })
    }
}

/// A weight with its unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OrderWeight {
    unit: OrderWeightUnit,
    value: u32,
}

impl OrderWeight {
    /// Validates a weight: 1 to 999 999 999.
    pub fn new(value: u32, unit: OrderWeightUnit) -> Result<Self, ValidationError> {
        if !(1..=999_999_999).contains(&value) {
            return Err(ValidationError::new(
                "weight",
                format!("{value} is outside 1..=999999999"),
            ));
        }
        Ok(Self { unit, value })
    }
}

/// The size and weight of a product.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProductProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<Dimension>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length: Option<Dimension>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight: Option<OrderWeight>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<Dimension>,
}

impl ProductProperties {
    /// Properties without any value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the height.
    pub fn height(mut self, value: Dimension) -> Self {
        self.height = Some(value);
        self
    }

    /// Sets the length.
    pub fn length(mut self, value: Dimension) -> Self {
        self.length = Some(value);
        self
    }

    /// Sets the weight.
    pub fn weight(mut self, value: OrderWeight) -> Self {
        self.weight = Some(value);
        self
    }

    /// Sets the width.
    pub fn width(mut self, value: Dimension) -> Self {
        self.width = Some(value);
        self
    }
}

/// A description of a product in a language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductDescription {
    text: DescriptionText,
    language_code: LanguageCode,
}

impl ProductDescription {
    /// A description in a language.
    pub fn new(text: DescriptionText, language_code: LanguageCode) -> Self {
        Self {
            text,
            language_code,
        }
    }
}

/// A product on an order line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLineProduct {
    name: String,
    source_id: SourceId,
    #[serde(skip_serializing_if = "Option::is_none")]
    barcode: Option<ProductBarcode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    country_code_of_origin: Option<CountryCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    descriptions: Option<NonEmpty<ProductDescription, 200>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ean: Option<Ean>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hs_code: Option<HsCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    physical_properties: Option<ProductProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    purchase_price: Option<OrderMoney>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sales_price: Option<SalesPrice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sku: Option<String>,
}

impl OrderLineProduct {
    /// A product with the fields the API requires.
    pub fn new(name: impl Into<String>, source_id: SourceId) -> Self {
        Self {
            name: name.into(),
            source_id,
            barcode: None,
            country_code_of_origin: None,
            descriptions: None,
            ean: None,
            hs_code: None,
            physical_properties: None,
            purchase_price: None,
            sales_price: None,
            sku: None,
        }
    }

    /// Sets the barcode.
    pub fn barcode(mut self, value: ProductBarcode) -> Self {
        self.barcode = Some(value);
        self
    }

    /// Sets the country of origin.
    pub fn country_code_of_origin(mut self, value: CountryCode) -> Self {
        self.country_code_of_origin = Some(value);
        self
    }

    /// Sets the descriptions.
    pub fn descriptions(mut self, value: NonEmpty<ProductDescription, 200>) -> Self {
        self.descriptions = Some(value);
        self
    }

    /// Sets the EAN.
    pub fn ean(mut self, value: Ean) -> Self {
        self.ean = Some(value);
        self
    }

    /// Sets the harmonised system code.
    pub fn hs_code(mut self, value: HsCode) -> Self {
        self.hs_code = Some(value);
        self
    }

    /// Sets the size and weight.
    pub fn physical_properties(mut self, value: ProductProperties) -> Self {
        self.physical_properties = Some(value);
        self
    }

    /// Sets the purchase price.
    pub fn purchase_price(mut self, value: OrderMoney) -> Self {
        self.purchase_price = Some(value);
        self
    }

    /// Sets the sales price.
    pub fn sales_price(mut self, value: SalesPrice) -> Self {
        self.sales_price = Some(value);
        self
    }

    /// Sets the stock keeping unit.
    pub fn sku(mut self, value: impl Into<String>) -> Self {
        self.sku = Some(value.into());
        self
    }
}

/// A line of an order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLine {
    product: OrderLineProduct,
    quantity: u32,
    source_id: SourceId,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<OrderMoney>,
}

impl OrderLine {
    /// A line with the fields the API requires. The quantity has to be at least 1.
    pub fn new(
        product: OrderLineProduct,
        quantity: u32,
        source_id: SourceId,
    ) -> Result<Self, ValidationError> {
        if quantity == 0 {
            return Err(ValidationError::new("quantity", "must be at least 1"));
        }
        Ok(Self {
            product,
            quantity,
            source_id,
            price: None,
        })
    }

    /// Sets the price.
    pub fn price(mut self, value: OrderMoney) -> Self {
        self.price = Some(value);
        self
    }
}

/// The options of an order's shipment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrderShipmentOptions {
    flags: BTreeSet<OrderOption>,
    insurance: Option<OrderMoney>,
    return_contribution_fee: Option<OrderMoney>,
    custom_label_text: Option<CustomLabelText>,
}

impl OrderShipmentOptions {
    /// Options that change nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Switches an option on.
    pub fn flag(mut self, option: OrderOption) -> Self {
        self.flags.insert(option);
        self
    }

    /// Sets the insured amount.
    pub fn insurance(mut self, value: OrderMoney) -> Self {
        self.insurance = Some(value);
        self
    }

    /// Sets the return contribution fee.
    pub fn return_contribution_fee(mut self, value: OrderMoney) -> Self {
        self.return_contribution_fee = Some(value);
        self
    }

    /// Sets the text printed on the label.
    pub fn custom_label_text(mut self, value: CustomLabelText) -> Self {
        self.custom_label_text = Some(value);
        self
    }
}

impl Serialize for OrderShipmentOptions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        for flag in &self.flags {
            map.serialize_entry(flag.as_str(), &serde_json::json!({}))?;
        }
        if let Some(amount) = &self.insurance {
            map.serialize_entry("insurance", &serde_json::json!({"amount": amount}))?;
        }
        if let Some(amount) = &self.return_contribution_fee {
            map.serialize_entry(
                "returnContributionFee",
                &serde_json::json!({"amount": amount}),
            )?;
        }
        if let Some(text) = &self.custom_label_text {
            map.serialize_entry("customLabelText", &serde_json::json!({"text": text}))?;
        }
        map.end()
    }
}

/// A pick-up location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PickupSpot {
    name: PickupName,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<PickupCode>,
}

impl PickupSpot {
    /// A location with a name.
    pub fn new(name: PickupName) -> Self {
        Self { name, code: None }
    }

    /// Sets the location code.
    pub fn code(mut self, value: PickupCode) -> Self {
        self.code = Some(value);
        self
    }
}

/// What kind of pick-up location a carrier uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickupKind {
    /// A location of any carrier except bpost and PostNL.
    General,
    /// A PostNL location, with its retail network.
    PostNl(RetailNetwork),
    /// A bpost location.
    Bpost(BpostPickupType),
}

/// Where the recipient collects an order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderPickup {
    /// The address of the location.
    pub address: OrderAddress,
    /// The location.
    pub location: PickupSpot,
    /// The kind of location.
    pub kind: PickupKind,
}

impl Serialize for OrderPickup {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut location =
            serde_json::to_value(&self.location).map_err(serde::ser::Error::custom)?;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("address", &self.address)?;
        match self.kind {
            PickupKind::General => {}
            PickupKind::PostNl(network) => map.serialize_entry("retailNetworkId", &network)?,
            PickupKind::Bpost(kind) => location["type"] = Value::String(kind.as_str().to_owned()),
        }
        map.serialize_entry("location", &location)?;
        map.end()
    }
}

/// Settings of the delivery.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeliverySettings {
    delivery_type: Option<OrderDeliveryType>,
    notify_on_delivery: bool,
    preferred_date: Option<time::Date>,
}

impl DeliverySettings {
    /// Settings that change nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the delivery type.
    pub fn delivery_type(mut self, value: OrderDeliveryType) -> Self {
        self.delivery_type = Some(value);
        self
    }

    /// Notifies the recipient on delivery.
    pub fn notify_on_delivery(mut self) -> Self {
        self.notify_on_delivery = true;
        self
    }

    /// Sets the preferred delivery date.
    pub fn preferred_date(mut self, value: time::Date) -> Self {
        self.preferred_date = Some(value);
        self
    }
}

impl Serialize for DeliverySettings {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        if let Some(kind) = &self.delivery_type {
            map.serialize_entry("type", kind)?;
        }
        if self.notify_on_delivery || self.preferred_date.is_some() {
            let mut options = serde_json::Map::new();
            if self.notify_on_delivery {
                options.insert("notifyOnDelivery".into(), serde_json::json!({}));
            }
            if let Some(date) = self.preferred_date {
                let text = date
                    .format(time::macros::format_description!("[year]-[month]-[day]"))
                    .map_err(serde::ser::Error::custom)?;
                options.insert("preferredDate".into(), serde_json::json!({"date": text}));
            }
            map.serialize_entry("options", &options)?;
        }
        map.end()
    }
}

/// How an order is shipped.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shipping {
    #[serde(skip_serializing_if = "Option::is_none")]
    additional_context: Option<AdditionalContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    carrier: Option<CarrierV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delivery: Option<DeliverySettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OrderShipmentOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_type: Option<PackageTypeV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pickup: Option<OrderPickup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipient: Option<OrderRecipient>,
}

impl Shipping {
    /// Shipping without any choice made.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets additional context.
    pub fn additional_context(mut self, value: AdditionalContext) -> Self {
        self.additional_context = Some(value);
        self
    }

    /// Sets the carrier.
    pub fn carrier(mut self, value: CarrierV2) -> Self {
        self.carrier = Some(value);
        self
    }

    /// Sets the delivery settings.
    pub fn delivery(mut self, value: DeliverySettings) -> Self {
        self.delivery = Some(value);
        self
    }

    /// Sets the options.
    pub fn options(mut self, value: OrderShipmentOptions) -> Self {
        self.options = Some(value);
        self
    }

    /// Sets the package type.
    pub fn package_type(mut self, value: PackageTypeV2) -> Self {
        self.package_type = Some(value);
        self
    }

    /// Sets the pick-up location.
    pub fn pickup(mut self, value: OrderPickup) -> Self {
        self.pickup = Some(value);
        self
    }

    /// Sets the recipient.
    pub fn recipient(mut self, value: OrderRecipient) -> Self {
        self.recipient = Some(value);
        self
    }
}

/// An order to push to MyParcel.
///
/// Pushing an order again with the same source id updates it. MyParcel accepts updates but does
/// not act on them yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    #[serde(with = "time::serde::rfc3339")]
    ordered_at: OffsetDateTime,
    source_id: SourceId,
    status: OrderStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    billing_details: Option<BillingDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    customer_reference: Option<CustomerReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines: Option<NonEmpty<OrderLine, 100>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shipping: Option<Shipping>,
}

impl Order {
    /// An order with the fields the API requires. Every order needs a status, because one
    /// order without it makes the API refuse the whole request.
    pub fn new(ordered_at: OffsetDateTime, source_id: SourceId, status: OrderStatus) -> Self {
        Self {
            ordered_at,
            source_id,
            status,
            billing_details: None,
            customer_reference: None,
            lines: None,
            shipping: None,
        }
    }

    /// Sets the billing details.
    pub fn billing_details(mut self, value: BillingDetails) -> Self {
        self.billing_details = Some(value);
        self
    }

    /// Sets your reference for the customer.
    pub fn customer_reference(mut self, value: CustomerReference) -> Self {
        self.customer_reference = Some(value);
        self
    }

    /// Sets the lines.
    pub fn lines(mut self, value: NonEmpty<OrderLine, 100>) -> Self {
        self.lines = Some(value);
        self
    }

    /// Sets the shipping.
    pub fn shipping(mut self, value: Shipping) -> Self {
        self.shipping = Some(value);
        self
    }
}

/// Between one and 50 orders, the most the API accepts in one request.
pub type OrderBatch = NonEmpty<Order, 50>;

/// A problem MyParcel reports for one order.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OrderProblem {
    /// The HTTP status of the problem.
    pub status: Option<u16>,
    /// A short summary.
    pub title: Option<String>,
    /// An explanation specific to this order.
    pub detail: Option<String>,
}

/// The result for one pushed order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderResult {
    /// The order was accepted. No order id is returned.
    Accepted,
    /// The order was refused.
    Problem(OrderProblem),
}

impl OrderResult {
    pub(crate) fn from_value(value: &Value) -> Self {
        let is_problem = value.get("title").is_some()
            || value.get("type").is_some()
            || value.get("detail").is_some();
        if !is_problem && value.get("status").and_then(Value::as_u64) == Some(202) {
            return Self::Accepted;
        }
        Self::Problem(
            serde_json::from_value(value.clone()).unwrap_or(OrderProblem {
                status: None,
                title: None,
                detail: None,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use time::macros::datetime;

    use super::*;

    fn address() -> OrderAddress {
        OrderAddress::new(
            OrderCity::new("Hoofddorp").unwrap(),
            CountryCode::NL,
            OrderStreet::new("Antareslaan").unwrap(),
        )
        .house_number(OrderHouseNumber::new("31").unwrap())
    }

    #[test]
    fn a_minimal_order_serializes_with_camel_case_names() {
        let order = Order::new(
            datetime!(2026-01-02 03:04:05 UTC),
            SourceId::new("ORDER-1").unwrap(),
            OrderStatus::Open,
        );
        assert_eq!(
            serde_json::to_value(&order).unwrap(),
            json!({"orderedAt": "2026-01-02T03:04:05Z", "sourceId": "ORDER-1", "status": "OPEN"})
        );
    }

    #[test]
    fn money_is_in_micros() {
        let price = OrderMoney::new(CurrencyCode::new("EUR").unwrap(), 24_950_000).unwrap();
        assert_eq!(
            serde_json::to_value(&price).unwrap(),
            json!({"currency": "EUR", "micros": 24950000})
        );
        assert!(
            OrderMoney::new(
                CurrencyCode::new("EUR").unwrap(),
                OrderMoney::MAX_MICROS + 1
            )
            .is_err()
        );
        assert!(SalesPrice::new(price, 101).is_err());
    }

    #[test]
    fn contacts_and_options_follow_the_schema() {
        let contact = RecipientContact::new(ContactKind::Private {
            name: ContactName::new("John").unwrap(),
        })
        .email_addresses(NonEmpty::one(ContactEmail::new("a@example.com").unwrap()));
        assert_eq!(
            serde_json::to_value(&contact).unwrap(),
            json!({"type": "PRIVATE", "name": "John", "emailAddresses": ["a@example.com"]})
        );
        let options = OrderShipmentOptions::new()
            .flag(OrderOption::RequiresSignature)
            .custom_label_text(CustomLabelText::new("hi").unwrap());
        assert_eq!(
            serde_json::to_value(&options).unwrap(),
            json!({"requiresSignature": {}, "customLabelText": {"text": "hi"}})
        );
    }

    #[test]
    fn pickups_carry_their_kind() {
        let pickup = OrderPickup {
            address: address(),
            location: PickupSpot::new(PickupName::new("Shop").unwrap()),
            kind: PickupKind::Bpost(BpostPickupType::ParcelLocker),
        };
        let value = serde_json::to_value(&pickup).unwrap();
        assert_eq!(value["location"]["type"], "PARCEL_LOCKER");
        let postnl = OrderPickup {
            kind: PickupKind::PostNl(RetailNetwork::Netherlands),
            ..pickup
        };
        assert_eq!(
            serde_json::to_value(&postnl).unwrap()["retailNetworkId"],
            "PNPNL-01"
        );
    }

    #[test]
    fn results_are_told_apart() {
        assert_eq!(
            OrderResult::from_value(&json!({"status": 202})),
            OrderResult::Accepted
        );
        match OrderResult::from_value(
            &json!({"type": "x", "title": "Bad", "status": 400, "detail": "no"}),
        ) {
            OrderResult::Problem(problem) => assert_eq!(problem.title.as_deref(), Some("Bad")),
            other => panic!("{other:?}"),
        }
    }
}
