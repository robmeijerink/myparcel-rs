//! Enumerations of the MyParcel API.

mod carrier;
mod delivery_type;
mod misc;
mod package_type;
mod status;

pub use carrier::{Carrier, CarrierV2};
pub use delivery_type::{DeliveryType, DeliveryTypeV2};
pub use misc::{
    CustomsContents, Direction, LabelQuadrant, LocationType, ShipmentType, TransactionType,
};
pub use package_type::{PackageType, PackageTypeV2};
pub use status::{MainStatus, ShipmentStatus, TransactionStatus};
