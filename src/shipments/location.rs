use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::country::CountryCode;
use crate::shipments::recipient::{BoxNumber, Region, State};

int_enum! {
    /// The kind of bpost pick-up point.
    pub enum BpostLocationType(u8) {
        /// Unknown.
        Unknown = 0,
        /// A post office.
        PostOffice = 1,
        /// A post point.
        PostPoint = 2,
        /// A parcel locker.
        ParcelLocker = 4,
        /// A click and collect point.
        ClickCollect = 8,
        /// A parcel point.
        ParcelPoint = 16,
    }
}

/// The kind of pick-up location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PickupLocationType {
    /// A retail location.
    Retail,
    /// A parcel locker.
    Locker,
    /// A post office.
    PostOffice,
    /// A bpost location.
    Bpost(BpostLocationType),
}

impl Serialize for PickupLocationType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Retail => serializer.serialize_str("retail"),
            Self::Locker => serializer.serialize_str("locker"),
            Self::PostOffice => serializer.serialize_str("post_office"),
            Self::Bpost(kind) => kind.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PickupLocationType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Name(String),
            Bpost(BpostLocationType),
        }

        match Wire::deserialize(deserializer)? {
            Wire::Bpost(kind) => Ok(Self::Bpost(kind)),
            Wire::Name(name) => match name.as_str() {
                "retail" => Ok(Self::Retail),
                "locker" => Ok(Self::Locker),
                "post_office" => Ok(Self::PostOffice),
                other => Err(serde::de::Error::custom(format!(
                    "unknown pick-up location type `{other}`"
                ))),
            },
        }
    }
}

/// A pick-up location the recipient collects the shipment from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pickup {
    cc: CountryCode,
    postal_code: String,
    city: String,
    street: String,
    number: String,
    location_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    number_suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box_number: Option<BoxNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<State>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location_type: Option<PickupLocationType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retail_network_id: Option<String>,
}

impl Pickup {
    /// A location with its address and name.
    pub fn new(
        cc: CountryCode,
        postal_code: impl Into<String>,
        city: impl Into<String>,
        street: impl Into<String>,
        number: impl Into<String>,
        location_name: impl Into<String>,
    ) -> Self {
        Self {
            cc,
            postal_code: postal_code.into(),
            city: city.into(),
            street: street.into(),
            number: number.into(),
            location_name: location_name.into(),
            number_suffix: None,
            box_number: None,
            region: None,
            state: None,
            location_code: None,
            location_type: None,
            retail_network_id: None,
        }
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

    /// Sets the state.
    pub fn state(mut self, value: State) -> Self {
        self.state = Some(value);
        self
    }

    /// Sets the location code.
    pub fn location_code(mut self, value: impl Into<String>) -> Self {
        self.location_code = Some(value.into());
        self
    }

    /// Sets the kind of location.
    pub fn location_type(mut self, value: PickupLocationType) -> Self {
        self.location_type = Some(value);
        self
    }

    /// Sets the retail network id.
    pub fn retail_network_id(mut self, value: impl Into<String>) -> Self {
        self.retail_network_id = Some(value.into());
        self
    }
}

/// A drop-off point where the shipment is handed in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DropOffPoint {
    location_code: String,
    location_name: String,
    postal_code: String,
    city: String,
    street: String,
    number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    number_suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<CountryCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retail_network_id: Option<String>,
}

impl DropOffPoint {
    /// A drop-off point with the fields the API requires.
    pub fn new(
        location_code: impl Into<String>,
        location_name: impl Into<String>,
        postal_code: impl Into<String>,
        city: impl Into<String>,
        street: impl Into<String>,
        number: impl Into<String>,
    ) -> Self {
        Self {
            location_code: location_code.into(),
            location_name: location_name.into(),
            postal_code: postal_code.into(),
            city: city.into(),
            street: street.into(),
            number: number.into(),
            number_suffix: None,
            box_number: None,
            region: None,
            state: None,
            cc: None,
            retail_network_id: None,
        }
    }

    /// Sets the house number suffix.
    pub fn number_suffix(mut self, value: impl Into<String>) -> Self {
        self.number_suffix = Some(value.into());
        self
    }

    /// Sets the box number.
    pub fn box_number(mut self, value: impl Into<String>) -> Self {
        self.box_number = Some(value.into());
        self
    }

    /// Sets the region.
    pub fn region(mut self, value: impl Into<String>) -> Self {
        self.region = Some(value.into());
        self
    }

    /// Sets the state.
    pub fn state(mut self, value: impl Into<String>) -> Self {
        self.state = Some(value.into());
        self
    }

    /// Sets the country.
    pub fn country(mut self, value: CountryCode) -> Self {
        self.cc = Some(value);
        self
    }

    /// Sets the retail network id.
    pub fn retail_network_id(mut self, value: impl Into<String>) -> Self {
        self.retail_network_id = Some(value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location_type_serializes_both_spec_variants() {
        assert_eq!(
            serde_json::to_string(&PickupLocationType::PostOffice).unwrap(),
            r#""post_office""#
        );
        assert_eq!(
            serde_json::to_string(&PickupLocationType::Bpost(BpostLocationType::ParcelLocker))
                .unwrap(),
            "4"
        );
    }
}
