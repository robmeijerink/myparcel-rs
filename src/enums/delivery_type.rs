int_enum! {
    /// When and where a shipment is delivered.
    pub enum DeliveryType(u8) {
        Morning = 1,
        Standard = 2,
        Evening = 3,
        Pickup = 4,
        SameDay = 6,
        Express = 7,
        EarlyMorning = 8,
    }
}
impl DeliveryType {
    /// The name used by the capabilities API.
    pub const fn v2_name(self) -> &'static str {
        match self {
            Self::Morning => "MORNING_DELIVERY",
            Self::Standard => "STANDARD_DELIVERY",
            Self::Evening => "EVENING_DELIVERY",
            Self::Pickup => "PICKUP_DELIVERY",
            Self::SameDay => "SAME_DAY_DELIVERY",
            Self::Express => "EXPRESS_DELIVERY",
            Self::EarlyMorning => "EARLY_MORNING_DELIVERY",
        }
    }

    /// The lower-case legacy name.
    pub const fn legacy_name(self) -> &'static str {
        match self {
            Self::Morning => "morning",
            Self::Standard => "standard",
            Self::Evening => "evening",
            Self::Pickup => "pickup",
            Self::SameDay => "same_day",
            Self::Express => "express",
            Self::EarlyMorning => "early_morning",
        }
    }

    /// The delivery type for a legacy name.
    pub fn from_legacy_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|kind| kind.legacy_name() == name)
    }
}

string_enum! {
    /// A delivery type as named by the capabilities API.
    pub enum DeliveryTypeV2 {
        Morning = "MORNING_DELIVERY",
        Standard = "STANDARD_DELIVERY",
        Evening = "EVENING_DELIVERY",
        Pickup = "PICKUP_DELIVERY",
        SameDay = "SAME_DAY_DELIVERY",
        Express = "EXPRESS_DELIVERY",
        EarlyMorning = "EARLY_MORNING_DELIVERY",
    }
}
impl From<DeliveryTypeV2> for DeliveryType {
    fn from(kind: DeliveryTypeV2) -> Self {
        match kind {
            DeliveryTypeV2::Morning => Self::Morning,
            DeliveryTypeV2::Standard => Self::Standard,
            DeliveryTypeV2::Evening => Self::Evening,
            DeliveryTypeV2::Pickup => Self::Pickup,
            DeliveryTypeV2::SameDay => Self::SameDay,
            DeliveryTypeV2::Express => Self::Express,
            DeliveryTypeV2::EarlyMorning => Self::EarlyMorning,
        }
    }
}

impl From<DeliveryType> for DeliveryTypeV2 {
    fn from(kind: DeliveryType) -> Self {
        match kind {
            DeliveryType::Morning => Self::Morning,
            DeliveryType::Standard => Self::Standard,
            DeliveryType::Evening => Self::Evening,
            DeliveryType::Pickup => Self::Pickup,
            DeliveryType::SameDay => Self::SameDay,
            DeliveryType::Express => Self::Express,
            DeliveryType::EarlyMorning => Self::EarlyMorning,
        }
    }
}
