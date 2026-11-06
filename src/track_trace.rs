//! Track and trace information.

use std::collections::BTreeMap;

use reqwest::Method;
use serde::Deserialize;

use crate::client::Client;
use crate::enums::{Carrier, MainStatus, ShipmentStatus};
use crate::error::Result;
use crate::http::{Context, Host, Operation, Request, Response};
use crate::reported::Reported;
use crate::types::{ShipmentId, ShipmentIds, join_ids};

/// Where an event took place.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Location {
    /// The name of the location.
    pub name: String,
    /// The country code.
    pub cc: Option<String>,
    /// The city.
    pub city: Option<String>,
    /// The postal code.
    pub postal_code: Option<String>,
    /// The street.
    pub street: Option<String>,
    /// The house number.
    pub number: Option<String>,
    /// The house number suffix.
    pub number_suffix: Option<String>,
    /// The longitude.
    pub longitude: Option<f64>,
    /// The latitude.
    pub latitude: Option<f64>,
}

/// One event in the history of a shipment.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Event {
    /// The carrier's event code.
    pub code: String,
    /// The status after the event.
    pub status: Option<Reported<ShipmentStatus>>,
    /// The simplified status after the event.
    pub main: Option<Reported<MainStatus>>,
    /// What happened.
    pub description: String,
    /// When it happened, as the API formats it.
    pub time: String,
    /// Whether the event means the shipment is delayed.
    pub delayed: bool,
    /// Where it happened.
    pub location: Location,
}

/// The status of a shipment in track and trace.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Status {
    /// The current status.
    pub current: Reported<ShipmentStatus>,
    /// The simplified status.
    pub main: Option<Reported<MainStatus>>,
    /// Whether this is the final status.
    #[serde(rename = "final")]
    pub is_final: bool,
}

/// The track and trace data of one shipment.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TrackTrace {
    /// The shipment.
    pub shipment_id: ShipmentId,
    /// The carrier.
    pub carrier: Reported<Carrier>,
    /// The code of the latest event.
    pub code: String,
    /// The description of the latest event.
    pub description: String,
    /// The time of the latest event, as the API formats it.
    pub time: String,
    /// The consumer portal link.
    pub link_consumer_portal: Option<String>,
    /// The carrier's tracking link.
    pub link_tracktrace: Option<String>,
    /// Whether the shipment is delayed.
    pub delayed: bool,
    /// The status.
    pub status: Status,
    /// The latest location.
    pub location: Option<Location>,
    /// Everything that happened so far.
    pub history: Vec<Event>,
}

pub(crate) struct Fetch {
    ids: String,
}

impl Fetch {
    pub fn new(ids: &ShipmentIds) -> Self {
        Self { ids: join_ids(ids) }
    }
}

impl Operation for Fetch {
    type Output = BTreeMap<ShipmentId, TrackTrace>;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Ok(Request::new(
            Method::GET,
            Host::Core,
            vec!["tracktraces".into(), self.ids.clone()],
            "application/json",
        ))
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        #[derive(Deserialize)]
        struct Envelope {
            data: Data,
        }
        #[derive(Deserialize)]
        struct Data {
            tracktraces: Vec<TrackTrace>,
        }

        let envelope: Envelope = response.ensure_success()?.json()?;
        Ok(envelope
            .data
            .tracktraces
            .into_iter()
            .map(|trace| (trace.shipment_id, trace))
            .collect())
    }
}

macro_rules! track_trace_methods {
    ($($async:ident)?; $($await:ident)?) => {
        /// Fetches track and trace data, keyed by shipment id.
        pub $($async)? fn fetch(
            &self,
            ids: &$crate::types::ShipmentIds,
        ) -> $crate::error::Result<
            ::std::collections::BTreeMap<$crate::types::ShipmentId, $crate::track_trace::TrackTrace>,
        > {
            self.client.execute($crate::track_trace::Fetch::new(ids)) $(.$await)?
        }
    };
}

#[cfg(feature = "blocking")]
pub(crate) use track_trace_methods;

/// Operations on track and trace data.
#[derive(Debug, Clone, Copy)]
pub struct TrackTraces<'a> {
    pub(crate) client: &'a Client,
}

impl TrackTraces<'_> {
    track_trace_methods!(async; await);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_final_flag() {
        let status: Status =
            serde_json::from_str(r#"{"current":7,"main":"delivered","final":true}"#).unwrap();
        assert!(status.is_final);
        assert_eq!(status.current, Reported::Known(ShipmentStatus::Delivered));
    }
}
