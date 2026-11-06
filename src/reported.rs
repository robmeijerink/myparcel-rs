use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// A value reported by the API.
///
/// MyParcel extends its enums regularly. A value this crate knows is returned as
/// [`Reported::Known`]; anything else is kept untouched as [`Reported::Unknown`] instead of failing
/// the whole response. Values sent to the API are always strictly typed.
#[derive(Debug, Clone, PartialEq)]
pub enum Reported<T> {
    /// A value this crate knows about.
    Known(T),
    /// The raw value of something this crate does not know about.
    Unknown(Value),
}

impl<T> Reported<T> {
    /// The known value, if there is one.
    pub fn known(&self) -> Option<&T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown(_) => None,
        }
    }

    /// Converts into the known value, if there is one.
    pub fn into_known(self) -> Option<T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown(_) => None,
        }
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Reported<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Value::deserialize(deserializer)?;
        Ok(match T::deserialize(&raw) {
            Ok(value) => Self::Known(value),
            Err(_) => Self::Unknown(raw),
        })
    }
}

impl<T: Serialize> Serialize for Reported<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Known(value) => value.serialize(serializer),
            Self::Unknown(raw) => raw.serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::Carrier;

    #[test]
    fn keeps_unknown_values() {
        let known: Reported<Carrier> = serde_json::from_str("1").unwrap();
        assert_eq!(known, Reported::Known(Carrier::PostNl));
        let unknown: Reported<Carrier> = serde_json::from_str("9999").unwrap();
        assert_eq!(unknown, Reported::Unknown(Value::from(9999)));
        assert_eq!(serde_json::to_string(&unknown).unwrap(), "9999");
    }
}
