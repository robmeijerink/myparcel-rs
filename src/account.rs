//! The account behind an API key.

use reqwest::Method;
use serde::Deserialize;

use crate::client::Client;
use crate::error::Result;
use crate::http::{Context, Host, Operation, Request, Response};
use crate::reported::Reported;

string_enum! {
    /// The MyParcel platform an account lives on.
    pub enum Platform {
        /// MyParcel Netherlands.
        MyparcelNl = "MYPARCEL_NL",
        /// MyParcel Belgium.
        MyparcelBe = "MYPARCEL_BE",
        /// MyParcel Italy.
        MyparcelIt = "MYPARCEL_IT",
    }
}

string_enum! {
    /// What an API key belongs to.
    pub enum PrincipalType {
        /// A shop.
        Shop = "SHOP",
        /// A user.
        User = "USER",
    }
}

string_enum! {
    /// A fulfilment platform.
    pub enum FulfilmentPlatform {
        /// Vasco.
        Vasco = "VASCO",
    }
}

string_enum! {
    /// The role of a principal.
    pub enum Role {
        /// A default shop.
        ShopDefault = "SHOP_DEFAULT",
        /// A shop of a sub customer.
        ShopSubCustomer = "SHOP_SUB_CUSTOMER",
        /// Customer service administrator.
        AdminCs = "ADMIN_CS",
        /// Advanced customer service administrator.
        AdminCsAdvanced = "ADMIN_CS_ADVANCED",
        /// Finance administrator.
        AdminFinance = "ADMIN_FINANCE",
        /// IT support administrator.
        AdminItSupport = "ADMIN_IT_SUPPORT",
        /// Sales administrator.
        AdminSales = "ADMIN_SALES",
        /// Super administrator.
        AdminSuper = "ADMIN_SUPER",
        /// Customer administrator.
        CustomerAdmin = "CUSTOMER_ADMIN",
        /// Customer financial.
        CustomerFinancial = "CUSTOMER_FINANCIAL",
        /// Customer functional manager.
        CustomerFunctionalManager = "CUSTOMER_FUNCTIONAL_MANAGER",
        /// Main customer user.
        CustomerMain = "CUSTOMER_MAIN",
        /// Customer marketing.
        CustomerMarketing = "CUSTOMER_MARKETING",
        /// Customer service.
        CustomerService = "CUSTOMER_SERVICE",
        /// Customer sub user.
        CustomerSub = "CUSTOMER_SUB",
        /// Customer warehouse.
        CustomerWarehouse = "CUSTOMER_WAREHOUSE",
    }
}

string_enum! {
    /// A feature of an account.
    pub enum Feature {
        /// Order management.
        OrderManagement = "ORDER_MANAGEMENT",
        /// Notification center.
        NotificationCenter = "NOTIFICATION_CENTER",
        /// The legacy order management.
        LegacyOrderManagement = "LEGACY_ORDER_MANAGEMENT",
        /// Automatic order import.
        AutoOrderImport = "AUTO_ORDER_IMPORT",
        /// bol.com VVB shipments.
        BolDotComVvbShipments = "BOL_DOT_COM_VVB_SHIPMENTS",
        /// Custom contracts.
        CustomContracts = "CUSTOM_CONTRACTS",
        /// Direct printing.
        DirectPrinting = "DIRECT_PRINTING",
        /// Disable the label headline.
        DisableLabelHeadline = "DISABLE_LABEL_HEADLINE",
        /// Extended shipping rules.
        ExtendedShippingRules = "EXTENDED_SHIPPING_RULES",
        /// Nested shipment label on the packing slip.
        NestedShipmentLabelOnPackingSlip = "NESTED_SHIPMENT_LABEL_ON_PACKING_SLIP",
        /// Order invoice.
        OrderInvoice = "ORDER_INVOICE",
        /// Order notes.
        OrderNotes = "ORDER_NOTES",
        /// Order picklist.
        OrderPicklist = "ORDER_PICKLIST",
        /// Return link on the packing slip.
        ReturnLinkOnPackingSlip = "RETURN_LINK_ON_PACKING_SLIP",
        /// Save the order sorting setting.
        SaveOrderSortingSetting = "SAVE_ORDER_SORTING_SETTING",
        /// Scan mode.
        ScanMode = "SCAN_MODE",
        /// Bulk create shipments from the address book.
        ShipmentBulkCreateFromAddressBook = "SHIPMENT_BULK_CREATE_FROM_ADDRESS_BOOK",
        /// Bulk edit shipments.
        ShipmentBulkEdits = "SHIPMENT_BULK_EDITS",
        /// Shipment discount.
        ShipmentDiscount = "SHIPMENT_DISCOUNT",
        /// Shop logo on the packing slip.
        ShopLogoOnPackingSlip = "SHOP_LOGO_ON_PACKING_SLIP",
        /// Analytics tier 1.
        Tier1Analytics = "TIER1_ANALYTICS",
        /// Analytics tier 2.
        Tier2Analytics = "TIER2_ANALYTICS",
        /// Analytics tier 3.
        Tier3Analytics = "TIER3_ANALYTICS",
        /// Analytics tier 4.
        Tier4Analytics = "TIER4_ANALYTICS",
        /// Secondary recipients of track and trace emails.
        TrackTraceEmailSecondaryRecipients = "TRACK_TRACE_EMAIL_SECONDARY_RECIPIENTS",
        /// Unlimited shipments per order line.
        UnlimitedShipmentsPerOrderLine = "UNLIMITED_SHIPMENTS_PER_ORDER_LINE",
        /// Unlimited shipping rules.
        UnlimitedShippingRules = "UNLIMITED_SHIPPING_RULES",
    }
}

/// What an API key belongs to, as reported by the IAM API.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Principal {
    /// The account.
    pub account_id: String,
    /// The platform.
    pub platform: Reported<Platform>,
    /// The features of the account.
    #[serde(default)]
    pub features: Vec<Reported<Feature>>,
    /// The fulfilment platforms of the account.
    #[serde(default)]
    pub fulfilment_platforms: Vec<Reported<FulfilmentPlatform>>,
    /// The id of the shop or user.
    pub id: String,
    /// Whether the key belongs to a shop or a user.
    #[serde(rename = "type")]
    pub kind: Reported<PrincipalType>,
    /// The role.
    pub role: Reported<Role>,
    /// The shops the key can act for. A shop key has exactly one.
    pub shop_ids: Vec<String>,
}

impl Principal {
    /// Whether the account has a feature.
    pub fn has_feature(&self, feature: Feature) -> bool {
        self.features
            .iter()
            .any(|reported| reported.known() == Some(&feature))
    }
}

pub(crate) struct Whoami;

impl Operation for Whoami {
    type Output = Principal;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Ok(Request::new(
            Method::GET,
            Host::Iam,
            vec!["whoami".into()],
            "application/json",
        ))
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        response.ensure_success()?.json()
    }
}

pub(crate) struct CheckKey;

impl Operation for CheckKey {
    type Output = bool;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Ok(Request::new(
            Method::GET,
            Host::Iam,
            vec!["whoami".into()],
            "application/json",
        ))
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        if response.status == 401 {
            return Ok(false);
        }
        response.ensure_success().map(|_| true)
    }
}

macro_rules! account_methods {
    ($($async:ident)?; $($await:ident)?) => {
        /// Fetches the principal behind the API key.
        pub $($async)? fn principal(&self) -> $crate::error::Result<$crate::account::Principal> {
            self.client.execute($crate::account::Whoami) $(.$await)?
        }

        /// Whether the API key is accepted.
        ///
        /// Only the status of the answer counts, so this does not depend on the shape of the
        /// principal. A rejected key gives `false`; any other failure is returned as an error.
        pub $($async)? fn is_valid(&self) -> $crate::error::Result<bool> {
            self.client.execute($crate::account::CheckKey) $(.$await)?
        }
    };
}

#[cfg(feature = "blocking")]
pub(crate) use account_methods;

/// Operations on the account.
#[derive(Debug, Clone, Copy)]
pub struct Account<'a> {
    pub(crate) client: &'a Client,
}

impl Account<'_> {
    account_methods!(async; await);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_principal() {
        let principal: Principal = serde_json::from_str(
            r#"{"accountId":"a1","platform":"MYPARCEL_NL","features":["ORDER_MANAGEMENT","NEW_THING"],"fulfilmentPlatforms":[],"id":"s1","type":"SHOP","role":"SHOP_DEFAULT","shopIds":["s1"]}"#,
        )
        .unwrap();
        assert!(principal.has_feature(Feature::OrderManagement));
        assert!(!principal.has_feature(Feature::ScanMode));
        assert_eq!(principal.features[1].known(), None);
        assert_eq!(principal.kind.known(), Some(&PrincipalType::Shop));
    }
}
