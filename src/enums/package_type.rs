int_enum! {
    /// The kind of package that is shipped.
    pub enum PackageType(u8) {
        Package = 1,
        Mailbox = 2,
        Unfranked = 3,
        DigitalStamp = 4,
        Pallet = 5,
        SmallPackage = 6,
        Envelope = 7,
    }
}
impl PackageType {
    /// The name used by the capabilities API.
    pub const fn v2_name(self) -> &'static str {
        match self {
            Self::Package => "PACKAGE",
            Self::Mailbox => "MAILBOX",
            Self::Unfranked => "UNFRANKED",
            Self::DigitalStamp => "DIGITAL_STAMP",
            Self::Pallet => "PALLET",
            Self::SmallPackage => "SMALL_PACKAGE",
            Self::Envelope => "ENVELOPE",
        }
    }

    /// The lower-case legacy name.
    ///
    /// `unfranked` is also accepted as `letter` and `small_package` as `package_small` by
    /// [`PackageType::from_legacy_name`].
    pub const fn legacy_name(self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Mailbox => "mailbox",
            Self::Unfranked => "letter",
            Self::DigitalStamp => "digital_stamp",
            Self::Pallet => "pallet",
            Self::SmallPackage => "package_small",
            Self::Envelope => "envelope",
        }
    }

    /// The package type for a legacy name.
    pub fn from_legacy_name(name: &str) -> Option<Self> {
        match name {
            "unfranked" => Some(Self::Unfranked),
            "small_package" => Some(Self::SmallPackage),
            _ => Self::ALL
                .iter()
                .copied()
                .find(|kind| kind.legacy_name() == name),
        }
    }
}

string_enum! {
    /// A package type as named by the capabilities API.
    pub enum PackageTypeV2 {
        Package = "PACKAGE",
        Mailbox = "MAILBOX",
        Unfranked = "UNFRANKED",
        DigitalStamp = "DIGITAL_STAMP",
        Pallet = "PALLET",
        SmallPackage = "SMALL_PACKAGE",
        Envelope = "ENVELOPE",
    }
}
impl From<PackageTypeV2> for PackageType {
    fn from(kind: PackageTypeV2) -> Self {
        match kind {
            PackageTypeV2::Package => Self::Package,
            PackageTypeV2::Mailbox => Self::Mailbox,
            PackageTypeV2::Unfranked => Self::Unfranked,
            PackageTypeV2::DigitalStamp => Self::DigitalStamp,
            PackageTypeV2::Pallet => Self::Pallet,
            PackageTypeV2::SmallPackage => Self::SmallPackage,
            PackageTypeV2::Envelope => Self::Envelope,
        }
    }
}

impl From<PackageType> for PackageTypeV2 {
    fn from(kind: PackageType) -> Self {
        match kind {
            PackageType::Package => Self::Package,
            PackageType::Mailbox => Self::Mailbox,
            PackageType::Unfranked => Self::Unfranked,
            PackageType::DigitalStamp => Self::DigitalStamp,
            PackageType::Pallet => Self::Pallet,
            PackageType::SmallPackage => Self::SmallPackage,
            PackageType::Envelope => Self::Envelope,
        }
    }
}
