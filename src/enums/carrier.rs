use crate::error::ValidationError;

int_enum! {
    /// A carrier, identified by the numeric id the shipments API uses.
    pub enum Carrier(u8) {
        PostNl = 1,
        Bpost = 2,
        CheapCargo = 3,
        Dpd = 4,
        Instabox = 5,
        DhlCheapCargo = 6,
        Bol = 7,
        Ups = 8,
        DhlForYou = 9,
        DhlParcelConnect = 10,
        DhlEuroplus = 11,
        UpsStandard = 12,
        UpsExpressSaver = 13,
        Gls = 14,
        Brt = 15,
        Trunkrs = 16,
        InPost = 17,
        PosteItaliane = 18,
        Spring = 19,
        ViaTim = 20,
        DhlFreight = 21,
    }
}
impl Carrier {
    /// The name used by the capabilities API, if this carrier has one.
    pub const fn v2_name(self) -> Option<&'static str> {
        match self {
            Self::PostNl => Some("POSTNL"),
            Self::Bpost => Some("BPOST"),
            Self::CheapCargo => Some("CHEAP_CARGO"),
            Self::Dpd => Some("DPD"),
            Self::Instabox => None,
            Self::DhlCheapCargo => None,
            Self::Bol => None,
            Self::Ups => None,
            Self::DhlForYou => Some("DHL_FOR_YOU"),
            Self::DhlParcelConnect => Some("DHL_PARCEL_CONNECT"),
            Self::DhlEuroplus => Some("DHL_EUROPLUS"),
            Self::UpsStandard => Some("UPS_STANDARD"),
            Self::UpsExpressSaver => Some("UPS_EXPRESS_SAVER"),
            Self::Gls => Some("GLS"),
            Self::Brt => Some("BRT"),
            Self::Trunkrs => Some("TRUNKRS"),
            Self::InPost => Some("INPOST"),
            Self::PosteItaliane => Some("POSTE_ITALIANE"),
            Self::Spring => Some("SPRING"),
            Self::ViaTim => Some("VIA_TIM"),
            Self::DhlFreight => Some("DHL_FREIGHT"),
        }
    }

    /// The lower-case name used in legacy query parameters.
    pub const fn legacy_name(self) -> &'static str {
        match self {
            Self::PostNl => "postnl",
            Self::Bpost => "bpost",
            Self::CheapCargo => "cheapcargo",
            Self::Dpd => "dpd",
            Self::Instabox => "instabox",
            Self::DhlCheapCargo => "dhl",
            Self::Bol => "bol.com",
            Self::Ups => "ups",
            Self::DhlForYou => "dhlforyou",
            Self::DhlParcelConnect => "dhlparcelconnect",
            Self::DhlEuroplus => "dhleuroplus",
            Self::UpsStandard => "upsstandard",
            Self::UpsExpressSaver => "upsexpresssaver",
            Self::Gls => "gls",
            Self::Brt => "brt",
            Self::Trunkrs => "trunkrs",
            Self::InPost => "inpost",
            Self::PosteItaliane => "posteitaliane",
            Self::Spring => "spring",
            Self::ViaTim => "viatim",
            Self::DhlFreight => "dhlfreight",
        }
    }

    /// The carrier for a capabilities API name.
    pub fn from_v2_name(name: &str) -> Option<Self> {
        CarrierV2::from_name(name).map(Self::from)
    }

    /// The carrier for a legacy lower-case name.
    pub fn from_legacy_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|carrier| carrier.legacy_name() == name)
    }
}

string_enum! {
    /// A carrier as named by the capabilities API.
    pub enum CarrierV2 {
        Bpost = "BPOST",
        Brt = "BRT",
        CheapCargo = "CHEAP_CARGO",
        DhlEuroplus = "DHL_EUROPLUS",
        DhlForYou = "DHL_FOR_YOU",
        DhlFreight = "DHL_FREIGHT",
        DhlParcelConnect = "DHL_PARCEL_CONNECT",
        Dpd = "DPD",
        Gls = "GLS",
        InPost = "INPOST",
        PosteItaliane = "POSTE_ITALIANE",
        PostNl = "POSTNL",
        Spring = "SPRING",
        Trunkrs = "TRUNKRS",
        UpsExpressSaver = "UPS_EXPRESS_SAVER",
        UpsStandard = "UPS_STANDARD",
        ViaTim = "VIA_TIM",
    }
}
impl From<CarrierV2> for Carrier {
    fn from(carrier: CarrierV2) -> Self {
        match carrier {
            CarrierV2::Bpost => Self::Bpost,
            CarrierV2::Brt => Self::Brt,
            CarrierV2::CheapCargo => Self::CheapCargo,
            CarrierV2::DhlEuroplus => Self::DhlEuroplus,
            CarrierV2::DhlForYou => Self::DhlForYou,
            CarrierV2::DhlFreight => Self::DhlFreight,
            CarrierV2::DhlParcelConnect => Self::DhlParcelConnect,
            CarrierV2::Dpd => Self::Dpd,
            CarrierV2::Gls => Self::Gls,
            CarrierV2::InPost => Self::InPost,
            CarrierV2::PosteItaliane => Self::PosteItaliane,
            CarrierV2::PostNl => Self::PostNl,
            CarrierV2::Spring => Self::Spring,
            CarrierV2::Trunkrs => Self::Trunkrs,
            CarrierV2::UpsExpressSaver => Self::UpsExpressSaver,
            CarrierV2::UpsStandard => Self::UpsStandard,
            CarrierV2::ViaTim => Self::ViaTim,
        }
    }
}

impl TryFrom<Carrier> for CarrierV2 {
    type Error = ValidationError;

    fn try_from(carrier: Carrier) -> Result<Self, Self::Error> {
        match carrier {
            Carrier::Bpost => Ok(Self::Bpost),
            Carrier::Brt => Ok(Self::Brt),
            Carrier::CheapCargo => Ok(Self::CheapCargo),
            Carrier::DhlEuroplus => Ok(Self::DhlEuroplus),
            Carrier::DhlForYou => Ok(Self::DhlForYou),
            Carrier::DhlFreight => Ok(Self::DhlFreight),
            Carrier::DhlParcelConnect => Ok(Self::DhlParcelConnect),
            Carrier::Dpd => Ok(Self::Dpd),
            Carrier::Gls => Ok(Self::Gls),
            Carrier::InPost => Ok(Self::InPost),
            Carrier::PosteItaliane => Ok(Self::PosteItaliane),
            Carrier::PostNl => Ok(Self::PostNl),
            Carrier::Spring => Ok(Self::Spring),
            Carrier::Trunkrs => Ok(Self::Trunkrs),
            Carrier::UpsExpressSaver => Ok(Self::UpsExpressSaver),
            Carrier::UpsStandard => Ok(Self::UpsStandard),
            Carrier::ViaTim => Ok(Self::ViaTim),
            other => Err(ValidationError::new(
                "carrier",
                format!("{other:?} has no capabilities name"),
            )),
        }
    }
}
