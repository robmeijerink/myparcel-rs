//! Validated primitives shared by the requests of the API.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;

use crate::error::ValidationError;

id_type! {
    /// Identifier of a shipment.
    ShipmentId
}

id_type! {
    /// Identifier of a shop.
    ShopId
}

id_type! {
    /// Identifier of an account.
    AccountId
}

id_type! {
    /// Identifier of a carrier contract.
    ContractId
}

id_type! {
    /// Identifier of a webhook subscription.
    WebhookSubscriptionId
}

bounded_string! {
    /// Identifier of a printer group of the MyParcel printing API.
    PrinterGroupId, "printer group id", 1, usize::MAX,
    |value| (!value.chars().all(|c| c.is_ascii_graphic()) || value.contains([';', ',', '"', '\\']))
        .then_some("must be printable ASCII without `;`, `,`, `\"` or `\\`")
}

bounded_string! {
    /// Your own reference for a shipment, at most 255 characters without `=` or `?`.
    ReferenceIdentifier, "reference identifier", 1, 255,
    |value| value.contains(['=', '?']).then_some("must not contain `=` or `?`")
}

bounded_string! {
    /// A label description of at most 45 characters.
    LabelDescription, "label description", 1, 45
}

/// A list with at least one and at most `MAX` items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NonEmpty<T, const MAX: usize = { usize::MAX }>(Vec<T>);

impl<T, const MAX: usize> NonEmpty<T, MAX> {
    /// Wraps a list, rejecting empty lists and lists that are too long.
    pub fn new(items: Vec<T>) -> Result<Self, ValidationError> {
        if items.is_empty() {
            return Err(ValidationError::new(
                "list",
                "must contain at least one item",
            ));
        }
        if items.len() > MAX {
            return Err(ValidationError::new(
                "list",
                format!("contains {} items, at most {MAX} are allowed", items.len()),
            ));
        }
        Ok(Self(items))
    }

    /// A list with a single item.
    pub fn one(item: T) -> Self {
        const { assert!(MAX >= 1) };
        Self(vec![item])
    }

    /// The items.
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }

    /// Iterates over the items.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }

    /// The number of items, which is at least one.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false`; a [`NonEmpty`] holds at least one item.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The items, consumed.
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }
}

impl<T, const MAX: usize> TryFrom<Vec<T>> for NonEmpty<T, MAX> {
    type Error = ValidationError;

    fn try_from(items: Vec<T>) -> Result<Self, Self::Error> {
        Self::new(items)
    }
}

impl<'a, T, const MAX: usize> IntoIterator for &'a NonEmpty<T, MAX> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T, const MAX: usize> IntoIterator for NonEmpty<T, MAX> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// One or more shipment ids.
pub type ShipmentIds = NonEmpty<ShipmentId>;

pub(crate) fn join_ids<T: fmt::Display, const MAX: usize>(ids: &NonEmpty<T, MAX>) -> String {
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(";")
}

/// A weight in grams, at most 999 999 999.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Grams(u32);

impl Grams {
    /// The largest weight the API accepts.
    pub const MAX: u32 = 999_999_999;

    /// Validates a weight.
    pub fn new(grams: u32) -> Result<Self, ValidationError> {
        if grams > Self::MAX {
            return Err(ValidationError::new(
                "weight",
                format!("{grams} grams is more than {}", Self::MAX),
            ));
        }
        Ok(Self(grams))
    }

    /// The weight in grams.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// An amount of euros in cents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cents(u64);

impl Cents {
    /// An amount in cents.
    pub const fn new(cents: u64) -> Self {
        Self(cents)
    }

    /// A whole number of euros.
    pub fn from_euros(euros: u64) -> Result<Self, ValidationError> {
        euros
            .checked_mul(100)
            .map(Self)
            .ok_or_else(|| ValidationError::new("amount", "is too large"))
    }

    /// The amount in cents.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// An amount of money in euros, serialized the way the shipments API expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Money {
    amount: Cents,
    currency: Currency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Currency;

impl Serialize for Cents {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0)
    }
}

impl Serialize for Currency {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("EUR")
    }
}

impl Money {
    /// An amount in euros.
    pub const fn eur(amount: Cents) -> Self {
        Self {
            amount,
            currency: Currency,
        }
    }

    /// The amount in cents.
    pub const fn amount(self) -> Cents {
        self.amount
    }
}

/// The insured amount of a shipment, at most 25 000 euros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Insurance(Money);

impl Insurance {
    /// The largest insured amount the API accepts, in cents.
    pub const MAX_CENTS: u64 = 2_500_000;

    /// Validates an insured amount.
    pub fn new(amount: Cents) -> Result<Self, ValidationError> {
        if amount.get() > Self::MAX_CENTS {
            return Err(ValidationError::new(
                "insurance",
                format!("{} cents is more than {}", amount.get(), Self::MAX_CENTS),
            ));
        }
        Ok(Self(Money::eur(amount)))
    }

    /// The insured amount.
    pub const fn amount(self) -> Cents {
        self.0.amount()
    }
}

const DATE_TIME: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

/// A date and time in the `YYYY-MM-DD HH:MM:SS` format of the shipments API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DateTime(PrimitiveDateTime);

impl DateTime {
    /// Wraps a date and time.
    pub const fn new(value: PrimitiveDateTime) -> Self {
        Self(value)
    }

    /// The wrapped value.
    pub const fn get(self) -> PrimitiveDateTime {
        self.0
    }
}

impl From<PrimitiveDateTime> for DateTime {
    fn from(value: PrimitiveDateTime) -> Self {
        Self(value)
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self.0.format(DATE_TIME).map_err(|_| fmt::Error)?;
        f.write_str(&text)
    }
}

impl FromStr for DateTime {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PrimitiveDateTime::parse(s, DATE_TIME)
            .map(Self)
            .map_err(|_| {
                ValidationError::new("date time", format!("`{s}` is not `YYYY-MM-DD HH:MM:SS`"))
            })
    }
}

impl Serialize for DateTime {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_empty_enforces_both_bounds() {
        assert!(NonEmpty::<u8, 2>::new(vec![]).is_err());
        assert!(NonEmpty::<u8, 2>::new(vec![1, 2]).is_ok());
        assert!(NonEmpty::<u8, 2>::new(vec![1, 2, 3]).is_err());
    }

    #[test]
    fn ids_are_joined_with_semicolons() {
        let ids = ShipmentIds::new(vec![ShipmentId(1), ShipmentId(22)]).unwrap();
        assert_eq!(join_ids(&ids), "1;22");
    }

    #[test]
    fn reference_identifier_rules() {
        assert!(ReferenceIdentifier::new("order-42").is_ok());
        assert!(ReferenceIdentifier::new("a=b").is_err());
        assert!(ReferenceIdentifier::new("a?b").is_err());
        assert!(ReferenceIdentifier::new("").is_err());
        assert!(ReferenceIdentifier::new("x".repeat(256)).is_err());
    }

    #[test]
    fn lengths_count_characters_not_bytes() {
        assert!(LabelDescription::new("é".repeat(45)).is_ok());
        assert!(LabelDescription::new("é".repeat(46)).is_err());
    }

    #[test]
    fn insurance_is_capped() {
        assert!(Insurance::new(Cents::new(2_500_000)).is_ok());
        assert!(Insurance::new(Cents::new(2_500_001)).is_err());
        let json = serde_json::to_string(&Insurance::new(Cents::from_euros(100).unwrap()).unwrap())
            .unwrap();
        assert_eq!(json, r#"{"amount":10000,"currency":"EUR"}"#);
    }

    #[test]
    fn grams_are_capped() {
        assert!(Grams::new(999_999_999).is_ok());
        assert!(Grams::new(1_000_000_000).is_err());
    }

    #[test]
    fn date_time_round_trips() {
        let value: DateTime = "2026-11-06 09:30:00".parse().unwrap();
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            r#""2026-11-06 09:30:00""#
        );
        assert!("2026-11-06".parse::<DateTime>().is_err());
    }
}
