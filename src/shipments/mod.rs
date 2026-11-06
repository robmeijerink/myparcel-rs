//! Creating, finding and deleting shipments.

mod customs;
mod location;
mod new;
mod options;
mod query;
mod recipient;
pub mod response;

pub(crate) mod ops;

use crate::client::Client;

pub use customs::{
    CustomsDeclaration, CustomsItem, ExternalProductReference, InvoiceNumber, ItemQuantity,
};
pub use location::{BpostLocationType, DropOffPoint, Pickup, PickupLocationType};
pub use new::{GeneralSettings, NewShipment, PhysicalProperties, SecondaryShipment};
pub use ops::{CreatedShipment, CreatedShipments};
pub use options::{IncotermsRule, ShipmentOption, ShipmentOptions};
pub use query::{Barcode, Order, Page, PageSize, RegionFilter, ShipmentQuery, Sort};
pub use recipient::{
    Area, BoxNumber, City, Company, EmailAddress, HouseNumber, NumberSuffix, Person, Phone,
    PostalCode, Recipient, Region, Sender, State, Street, StreetAdditionalInfo,
};
pub use response::Shipment;

use crate::types::NonEmpty;

/// Between one and 100 shipments, the most the API accepts in one request.
pub type ShipmentBatch = NonEmpty<NewShipment, 100>;

macro_rules! shipments_methods {
    ($($async:ident)?; $($await:ident)?) => {
        /// Creates shipments.
        ///
        /// A shipment without a reference gets a generated `sdk_` reference, so every created
        /// shipment can be matched to what was sent.
        pub $($async)? fn create(
            &self,
            shipments: &$crate::shipments::ShipmentBatch,
            format: Option<&$crate::labels::LabelFormat>,
        ) -> $crate::error::Result<$crate::shipments::CreatedShipments> {
            self.client.execute($crate::shipments::ops::Create::new(shipments, format)?) $(.$await)?
        }

        /// Creates shipments and sends their labels to a printer group.
        ///
        /// This does not print shipments that already exist.
        pub $($async)? fn print(
            &self,
            shipments: &$crate::shipments::ShipmentBatch,
            printer_group: &$crate::types::PrinterGroupId,
        ) -> $crate::error::Result<Vec<$crate::shipments::CreatedShipment>> {
            self.client.execute($crate::shipments::ops::Print::new(shipments, printer_group)?) $(.$await)?
        }

        /// Finds shipments with a query.
        pub $($async)? fn query(
            &self,
            query: $crate::shipments::ShipmentQuery,
        ) -> $crate::error::Result<Vec<$crate::shipments::Shipment>> {
            self.client.execute($crate::shipments::ops::Query::new(query)) $(.$await)?
        }

        /// Finds one shipment by id.
        pub $($async)? fn find(
            &self,
            id: $crate::types::ShipmentId,
        ) -> $crate::error::Result<Option<$crate::shipments::Shipment>> {
            let ids = $crate::types::ShipmentIds::one(id);
            Ok(self.client.execute($crate::shipments::ops::Get::new(&ids)) $(.$await)? ?.into_iter().next())
        }

        /// Finds shipments by id.
        pub $($async)? fn find_many(
            &self,
            ids: &$crate::types::ShipmentIds,
        ) -> $crate::error::Result<Vec<$crate::shipments::Shipment>> {
            self.client.execute($crate::shipments::ops::Get::new(ids)) $(.$await)?
        }

        /// Finds shipments by reference. `*` works as a wildcard.
        ///
        /// No page size is sent, because the API returns nothing when a size of 200 or more is
        /// combined with a reference.
        pub $($async)? fn find_by_reference(
            &self,
            reference: &$crate::types::ReferenceIdentifier,
        ) -> $crate::error::Result<Vec<$crate::shipments::Shipment>> {
            let query = $crate::shipments::ShipmentQuery::new()
                .reference_identifier(reference)
                .without_size();
            self.client.execute($crate::shipments::ops::Query::new(query)) $(.$await)?
        }

        /// Deletes shipments. Only concepts can be deleted.
        pub $($async)? fn delete(&self, ids: &$crate::types::ShipmentIds) -> $crate::error::Result<()> {
            self.client.execute($crate::shipments::ops::Delete::new(ids)) $(.$await)?
        }
    };
}

#[cfg(feature = "blocking")]
pub(crate) use shipments_methods;

/// Operations on shipments.
#[derive(Debug, Clone, Copy)]
pub struct Shipments<'a> {
    pub(crate) client: &'a Client,
}

impl Shipments<'_> {
    shipments_methods!(async; await);
}
