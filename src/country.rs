//! Country codes and shipping zones.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};

use crate::error::ValidationError;

const ALL: &[&str] = &[
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AN", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX",
    "AZ", "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ", "BR",
    "BS", "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM",
    "CN", "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM", "DO", "DZ", "EC",
    "EE", "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR", "GA", "GB", "GD", "GE",
    "GF", "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK",
    "HM", "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR", "IS", "IT", "JE",
    "JM", "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KY", "KZ", "LA", "LB",
    "LC", "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH",
    "MK", "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ",
    "NA", "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF",
    "PG", "PH", "PK", "PL", "PM", "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU",
    "RW", "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR",
    "SS", "ST", "SV", "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN",
    "TO", "TR", "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG",
    "VI", "VN", "VU", "WF", "WS", "XK", "YE", "YT", "ZA", "ZM", "ZW",
];

const EU: &[&str] = &[
    "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "ES", "FI", "FR", "GR", "HR", "HU", "IE", "IT",
    "LT", "LU", "LV", "MT", "NL", "PL", "PT", "RO", "SE", "SI", "SK",
];

/// An ISO 3166-1 alpha-2 country code.
///
/// The accepted codes are the ISO codes plus `AN` and `XK`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CountryCode(&'static str);

/// Shipping zone used for insurance and carrier rules.
///
/// The Netherlands and Belgium are zones of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShippingZone {
    /// The Netherlands.
    Netherlands,
    /// Belgium.
    Belgium,
    /// Any other member of the European Union.
    Eu,
    /// The rest of the world.
    RestOfWorld,
}

impl CountryCode {
    /// The Netherlands.
    pub const NL: Self = Self("NL");
    /// Belgium.
    pub const BE: Self = Self("BE");

    /// Parses an upper-case ISO country code.
    pub fn new(code: &str) -> Result<Self, ValidationError> {
        ALL.binary_search(&code)
            .map(|index| Self(ALL[index]))
            .map_err(|_| {
                ValidationError::new(
                    "country code",
                    format!("`{code}` is not a supported country code"),
                )
            })
    }

    /// Every accepted country code.
    pub fn all() -> impl Iterator<Item = Self> {
        ALL.iter().map(|code| Self(code))
    }

    /// The two letter code.
    pub const fn as_str(self) -> &'static str {
        self.0
    }

    /// The shipping zone of this country.
    pub fn shipping_zone(self) -> ShippingZone {
        match self.0 {
            "NL" => ShippingZone::Netherlands,
            "BE" => ShippingZone::Belgium,
            code if EU.binary_search(&code).is_ok() => ShippingZone::Eu,
            _ => ShippingZone::RestOfWorld,
        }
    }

    /// Whether the country is in the `EU` zone.
    ///
    /// The Netherlands and Belgium are zones of their own, so this is `false` for both.
    pub fn is_eu(self) -> bool {
        self.shipping_zone() == ShippingZone::Eu
    }

    /// Whether the country is in the `ROW` zone.
    pub fn is_row(self) -> bool {
        self.shipping_zone() == ShippingZone::RestOfWorld
    }
}

impl fmt::Display for CountryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl FromStr for CountryCode {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Serialize for CountryCode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

impl<'de> Deserialize<'de> for CountryCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::new(&value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_are_sorted_for_binary_search() {
        assert!(ALL.windows(2).all(|w| w[0] < w[1]));
        assert!(EU.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn zones_treat_the_netherlands_and_belgium_apart() {
        assert_eq!(CountryCode::NL.shipping_zone(), ShippingZone::Netherlands);
        assert_eq!(CountryCode::BE.shipping_zone(), ShippingZone::Belgium);
        assert_eq!(
            CountryCode::new("DE").unwrap().shipping_zone(),
            ShippingZone::Eu
        );
        assert_eq!(
            CountryCode::new("MT").unwrap().shipping_zone(),
            ShippingZone::Eu
        );
        assert_eq!(
            CountryCode::new("XK").unwrap().shipping_zone(),
            ShippingZone::RestOfWorld
        );
        assert!(!CountryCode::NL.is_eu());
    }

    #[test]
    fn rejects_unknown_and_lowercase_codes() {
        assert!(CountryCode::new("nl").is_err());
        assert!(CountryCode::new("ZZ").is_err());
        assert!(CountryCode::new("").is_err());
    }

    #[test]
    fn contains_the_extra_codes() {
        assert_eq!(ALL.len(), 251);
        assert!(CountryCode::new("AN").is_ok());
    }
}
