//! Webhook subscriptions.

use reqwest::Method;
use serde::Deserialize;
use serde_json::json;
use url::Url;

use crate::client::Client;
use crate::error::{Error, Result, ValidationError};
use crate::http::{Context, Host, Operation, Request, Response};
use crate::reported::Reported;
use crate::types::{AccountId, NonEmpty, ShopId, WebhookSubscriptionId, join_ids};

const JSON_CONTENT_TYPE: &str = "application/json;version=1.1";
const SUBSCRIPTION_ACCEPT: &str =
    "application/json;charset=utf-8;version=1.1,application/json;charset=utf-8,application/json";

string_enum! {
    /// The event a webhook is called for.
    pub enum Hook {
        /// A shipment changes status. This is the one MyParcel recommends.
        ShipmentStatusChange = "shipment_status_change",
        /// A label is created or printed.
        ShipmentLabelCreated = "shipment_label_created",
        /// An order changes status.
        OrderStatusChange = "order_status_change",
        /// The carriers available to a shop change.
        ShopCarrierAccessibilityUpdated = "shop_carrier_accessibility_updated",
        /// A shop is updated.
        ShopUpdated = "shop_updated",
    }
}

/// The url a webhook is delivered to. MyParcel only delivers to `https` urls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookUrl(Url);

impl WebhookUrl {
    /// Validates a url: it must use `https`, have a host and carry no credentials.
    pub fn new(url: Url) -> std::result::Result<Self, ValidationError> {
        if url.scheme() != "https" {
            return Err(ValidationError::new("webhook url", "must use https"));
        }
        if url.host_str().is_none() {
            return Err(ValidationError::new("webhook url", "must have a host"));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(ValidationError::new(
                "webhook url",
                "must not contain credentials",
            ));
        }
        Ok(Self(url))
    }

    /// The url.
    pub fn as_url(&self) -> &Url {
        &self.0
    }
}

impl std::str::FromStr for WebhookUrl {
    type Err = ValidationError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let url = Url::parse(s)
            .map_err(|error| ValidationError::new("webhook url", error.to_string()))?;
        Self::new(url)
    }
}

/// A webhook subscription as reported by the API.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Subscription {
    /// The id, needed to unsubscribe.
    pub id: WebhookSubscriptionId,
    /// The account.
    pub account_id: AccountId,
    /// The shop.
    pub shop_id: ShopId,
    /// The event.
    pub hook: Reported<Hook>,
    /// The url the webhook is delivered to.
    pub url: String,
    /// The payload setting, when there is one.
    pub payload: Option<String>,
}

pub(crate) struct Subscribe {
    body: serde_json::Value,
}

impl Subscribe {
    pub fn new(hook: Hook, url: &WebhookUrl) -> Self {
        Self {
            body: json!({"data": {"webhook_subscriptions": [{"hook": hook, "url": url.as_url().as_str()}]}}),
        }
    }
}

impl Operation for Subscribe {
    type Output = WebhookSubscriptionId;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Request::new(
            Method::POST,
            Host::Core,
            vec!["webhook_subscriptions".into()],
            "application/json",
        )
        .json_body(JSON_CONTENT_TYPE, &self.body)
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        #[derive(Deserialize)]
        struct Envelope {
            data: Data,
        }
        #[derive(Deserialize)]
        struct Data {
            ids: Vec<Id>,
        }
        #[derive(Deserialize)]
        struct Id {
            id: WebhookSubscriptionId,
        }

        let envelope: Envelope = response.ensure_success()?.json()?;
        envelope
            .data
            .ids
            .into_iter()
            .next()
            .map(|item| item.id)
            .ok_or(Error::UnexpectedResponse("no subscription id was returned"))
    }
}

pub(crate) struct Unsubscribe {
    ids: String,
}

impl Unsubscribe {
    pub fn new(ids: &NonEmpty<WebhookSubscriptionId>) -> Self {
        Self { ids: join_ids(ids) }
    }
}

impl Operation for Unsubscribe {
    type Output = ();

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Ok(Request::new(
            Method::DELETE,
            Host::Core,
            vec!["webhook_subscriptions".into(), self.ids.clone()],
            "application/json",
        ))
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        response.ensure_success().map(drop)
    }
}

pub(crate) struct List {
    ids: Option<String>,
    hook: Option<Hook>,
}

impl List {
    pub fn all(hook: Option<Hook>) -> Self {
        Self { ids: None, hook }
    }

    pub fn by_id(ids: &NonEmpty<WebhookSubscriptionId>) -> Self {
        Self {
            ids: Some(join_ids(ids)),
            hook: None,
        }
    }
}

impl Operation for List {
    type Output = Vec<Subscription>;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        let mut segments = vec!["webhook_subscriptions".to_owned()];
        segments.extend(self.ids.clone());
        let mut request = Request::new(Method::GET, Host::Core, segments, SUBSCRIPTION_ACCEPT);
        if let Some(hook) = self.hook {
            request = request.query("hook", hook.as_str());
        }
        Ok(request)
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        #[derive(Deserialize)]
        struct Envelope {
            data: Data,
        }
        #[derive(Deserialize)]
        struct Data {
            webhook_subscriptions: Vec<Subscription>,
        }

        Ok(response
            .ensure_success()?
            .json::<Envelope>()?
            .data
            .webhook_subscriptions)
    }
}

macro_rules! webhooks_methods {
    ($($async:ident)?; $($await:ident)?) => {
        /// Subscribes a url to a hook and returns the id of the subscription.
        ///
        /// There is one subscription per hook: subscribing again replaces the earlier one.
        /// Keep the id; without it a subscription can only be found through
        /// [`list`](Self::list).
        pub $($async)? fn subscribe(
            &self,
            hook: $crate::webhooks::Hook,
            url: &$crate::webhooks::WebhookUrl,
        ) -> $crate::error::Result<$crate::types::WebhookSubscriptionId> {
            self.client.execute($crate::webhooks::Subscribe::new(hook, url)) $(.$await)?
        }

        /// Removes subscriptions.
        pub $($async)? fn unsubscribe(
            &self,
            ids: &$crate::types::NonEmpty<$crate::types::WebhookSubscriptionId>,
        ) -> $crate::error::Result<()> {
            self.client.execute($crate::webhooks::Unsubscribe::new(ids)) $(.$await)?
        }

        /// Lists the subscriptions, optionally for one hook.
        pub $($async)? fn list(
            &self,
            hook: Option<$crate::webhooks::Hook>,
        ) -> $crate::error::Result<Vec<$crate::webhooks::Subscription>> {
            self.client.execute($crate::webhooks::List::all(hook)) $(.$await)?
        }

        /// Fetches subscriptions by id.
        pub $($async)? fn get(
            &self,
            ids: &$crate::types::NonEmpty<$crate::types::WebhookSubscriptionId>,
        ) -> $crate::error::Result<Vec<$crate::webhooks::Subscription>> {
            self.client.execute($crate::webhooks::List::by_id(ids)) $(.$await)?
        }
    };
}

#[cfg(feature = "blocking")]
pub(crate) use webhooks_methods;

/// Operations on webhook subscriptions.
#[derive(Debug, Clone, Copy)]
pub struct Webhooks<'a> {
    pub(crate) client: &'a Client,
}

impl Webhooks<'_> {
    webhooks_methods!(async; await);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_must_be_https_without_credentials() {
        assert!("https://example.com/hook".parse::<WebhookUrl>().is_ok());
        assert!("http://example.com/hook".parse::<WebhookUrl>().is_err());
        assert!(
            "https://user:pw@example.com/hook"
                .parse::<WebhookUrl>()
                .is_err()
        );
    }

    #[test]
    fn unknown_hooks_are_kept_when_reported() {
        let subscription: Subscription = serde_json::from_str(
            r#"{"id":1,"account_id":2,"shop_id":3,"hook":"something_new","url":"https://x.test/"}"#,
        )
        .unwrap();
        assert!(subscription.hook.known().is_none());
    }
}
