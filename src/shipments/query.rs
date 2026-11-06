use time::Date;
use time::macros::format_description;

use crate::enums::{Carrier, PackageType, ShipmentStatus, ShipmentType, TransactionStatus};
use crate::error::ValidationError;
use crate::types::{NonEmpty, ReferenceIdentifier, ShopId};

bounded_string! {
    /// A carrier barcode of 6 to 32 letters, digits, dashes and whitespace.
    Barcode, "barcode", 6, 32,
    |value| (!value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c.is_whitespace()))
        .then_some("may only contain letters, digits, dashes and whitespace")
}

string_enum! {
    /// The order of query results.
    pub enum Order {
        /// Ascending.
        Ascending = "ASC",
        /// Descending.
        Descending = "DESC",
    }
}

string_enum! {
    /// The region grouping of a shipment.
    pub enum RegionFilter {
        /// The Netherlands.
        Nl = "NL",
        /// Belgium.
        Be = "BE",
        /// Italy.
        It = "IT",
        /// The European Union.
        Eu = "EU",
        /// The rest of the world.
        World = "WERELD",
    }
}

string_enum! {
    /// A field shipments can be sorted by.
    pub enum Sort {
        /// The amount.
        Amount = "amount",
        /// The creation time.
        Created = "created",
        /// Who created the shipment.
        CreatedBy = "created_by",
        /// The currency.
        Currency = "currency",
        /// The drop-off point.
        DropOffPoint = "drop_off_point",
        /// Whether the shipment is hidden.
        Hidden = "hidden",
        /// The id.
        Id = "id",
        /// Whether the shipment was marked delivered.
        MarkDelivered = "mark_delivered",
        /// The modification time.
        Modified = "modified",
        /// Who modified the shipment.
        ModifiedBy = "modified_by",
        /// The order shipment identifier.
        OrderShipmentIdentifier = "order_shipment_identifier",
        /// The parent shipment.
        ParentId = "parent_id",
        /// The recipient.
        Recipient = "recipient",
        /// The reference identifier.
        ReferenceIdentifier = "reference_identifier",
        /// The sender.
        Sender = "sender",
        /// The shipment type.
        ShipmentType = "shipment_type",
        /// The shop.
        ShopId = "shop_id",
        /// The status.
        Status = "status",
        /// The transaction status.
        TransactionStatus = "transaction_status",
    }
}

/// A page number from 1 to 1000.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page(u16);

impl Page {
    /// Validates a page number.
    pub fn new(page: u16) -> Result<Self, ValidationError> {
        if !(1..=1000).contains(&page) {
            return Err(ValidationError::new(
                "page",
                format!("{page} is outside 1..=1000"),
            ));
        }
        Ok(Self(page))
    }
}

/// The number of results per page, from 0 to 10 000.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSize(u16);

impl PageSize {
    /// The size used when none is given: 300.
    pub const DEFAULT: Self = Self(300);

    /// Validates a page size.
    pub fn new(size: u16) -> Result<Self, ValidationError> {
        if size > 10_000 {
            return Err(ValidationError::new(
                "page size",
                format!("{size} is more than 10000"),
            ));
        }
        Ok(Self(size))
    }
}

/// Filters for finding shipments.
///
/// A query asks for [`PageSize::DEFAULT`] results per page unless told otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipmentQuery {
    params: Vec<(&'static str, String)>,
    size: Option<PageSize>,
}

fn bool_param(value: bool) -> String {
    u8::from(value).to_string()
}

fn list<T>(items: &NonEmpty<T>, value: impl Fn(&T) -> String) -> String {
    items.iter().map(value).collect::<Vec<_>>().join(";")
}

fn date(value: Date) -> String {
    value
        .format(format_description!("[year]-[month]-[day]"))
        .expect("a date always formats")
}

impl Default for ShipmentQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl ShipmentQuery {
    /// A query without filters.
    pub fn new() -> Self {
        Self {
            params: Vec::new(),
            size: Some(PageSize::DEFAULT),
        }
    }

    fn with(mut self, name: &'static str, value: String) -> Self {
        self.params.retain(|(existing, _)| *existing != name);
        self.params.push((name, value));
        self
    }

    /// Only shipments with this barcode.
    pub fn barcode(self, value: &Barcode) -> Self {
        self.with("barcode", value.to_string())
    }

    /// Only shipments of these carriers.
    pub fn carrier(self, value: &NonEmpty<Carrier>) -> Self {
        self.with("carrier_id", list(value, |c| c.value().to_string()))
    }

    /// Only shipments created on or after this date.
    pub fn created(self, value: Date) -> Self {
        self.with("created", date(value))
    }

    /// Only shipments created on or after this date.
    pub fn from(self, value: Date) -> Self {
        self.with("from", date(value))
    }

    /// Only shipments created up to this date.
    pub fn to(self, value: Date) -> Self {
        self.with("to", date(value))
    }

    /// Filters on whether the shipment is delayed.
    pub fn delayed(self, value: bool) -> Self {
        self.with("delayed", bool_param(value))
    }

    /// Filters on whether the shipment is delivered.
    pub fn delivered(self, value: bool) -> Self {
        self.with("delivered", bool_param(value))
    }

    /// Only shipments that have to be dropped off today.
    pub fn dropoff_today(self, value: bool) -> Self {
        self.with("dropoff_today", bool_param(value))
    }

    /// Only shipments with this external identifier.
    pub fn external_identifier(self, value: impl Into<String>) -> Self {
        self.with("external_identifier", value.into())
    }

    /// Hides shipments of hidden shops.
    pub fn filter_hidden_shops(self, value: bool) -> Self {
        self.with("filter_hidden_shops", bool_param(value))
    }

    /// Filters on whether the shipment is hidden.
    pub fn hidden(self, value: bool) -> Self {
        self.with("hidden", bool_param(value))
    }

    /// Includes the secondary shipments of multi-collo shipments.
    pub fn include_secondary_shipments(self, value: bool) -> Self {
        self.with("include_secondary_shipments", bool_param(value))
    }

    /// Adds the consumer portal link to the results.
    pub fn link_consumer_portal(self, value: bool) -> Self {
        self.with("link_consumer_portal", bool_param(value))
    }

    /// Sets the sort order.
    pub fn order(self, value: Order) -> Self {
        self.with("order", value.to_string())
    }

    /// Only shipments of these package types.
    pub fn package_type(self, value: &NonEmpty<PackageType>) -> Self {
        self.with("package_type", list(value, |p| p.value().to_string()))
    }

    /// Asks for a page of results.
    pub fn page(self, value: Page) -> Self {
        self.with("page", value.0.to_string())
    }

    /// Searches for a keyword.
    pub fn search(self, value: impl Into<String>) -> Self {
        self.with("q", value.into())
    }

    /// Only shipments with this reference. `*` works as a wildcard.
    pub fn reference_identifier(self, value: &ReferenceIdentifier) -> Self {
        self.with("reference_identifier", value.to_string())
    }

    /// Only shipments of this region.
    pub fn region(self, value: RegionFilter) -> Self {
        self.with("region", value.to_string())
    }

    /// Only shipments of these types.
    pub fn shipment_type(self, value: &NonEmpty<ShipmentType>) -> Self {
        self.with("shipment_type", list(value, |t| t.value().to_string()))
    }

    /// Only shipments of these shops.
    pub fn shop_id(self, value: &NonEmpty<ShopId>) -> Self {
        self.with("shop_id", list(value, ToString::to_string))
    }

    /// Sets the number of results per page.
    pub fn size(mut self, value: PageSize) -> Self {
        self.size = Some(value);
        self
    }

    /// Does not send a page size. The API misbehaves when a large size is combined with a
    /// reference filter, so [`crate::shipments::Shipments::find_by_reference`] uses this.
    pub fn without_size(mut self) -> Self {
        self.size = None;
        self
    }

    /// Sorts the results.
    pub fn sort(self, value: Sort) -> Self {
        self.with("sort", value.to_string())
    }

    /// Only shipments with these statuses.
    pub fn status(self, value: &NonEmpty<ShipmentStatus>) -> Self {
        self.with("status", list(value, |s| s.value().to_string()))
    }

    /// Only shipments with this transaction status.
    pub fn transaction_status(self, value: TransactionStatus) -> Self {
        self.with("transaction_status", value.to_string())
    }

    pub(crate) fn into_params(self) -> Vec<(&'static str, String)> {
        let mut params = self.params;
        if let Some(size) = self.size {
            params.push(("size", size.0.to_string()));
        }
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_a_page_size_of_300() {
        assert_eq!(
            ShipmentQuery::new().into_params(),
            vec![("size", "300".to_owned())]
        );
        assert!(ShipmentQuery::new().without_size().into_params().is_empty());
    }

    #[test]
    fn encodes_lists_and_flags() {
        let carriers = NonEmpty::new(vec![Carrier::PostNl, Carrier::Dpd]).unwrap();
        let params = ShipmentQuery::new()
            .carrier(&carriers)
            .delivered(true)
            .order(Order::Descending)
            .into_params();
        assert!(params.contains(&("carrier_id", "1;4".to_owned())));
        assert!(params.contains(&("delivered", "1".to_owned())));
        assert!(params.contains(&("order", "DESC".to_owned())));
    }

    #[test]
    fn a_filter_is_only_sent_once() {
        let params = ShipmentQuery::new()
            .delayed(true)
            .delayed(false)
            .into_params();
        assert_eq!(
            params.iter().filter(|(name, _)| *name == "delayed").count(),
            1
        );
    }

    #[test]
    fn bounds() {
        assert!(Page::new(0).is_err());
        assert!(Page::new(1001).is_err());
        assert!(PageSize::new(10_001).is_err());
        assert!(Barcode::new("3SABCD0123456").is_ok());
        assert!(Barcode::new("ab").is_err());
        assert!(Barcode::new("abc_def").is_err());
    }
}
