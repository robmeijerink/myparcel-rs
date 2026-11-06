//! Label formats and downloads.

use reqwest::Method;
use serde::Deserialize;
use time::OffsetDateTime;
use time::macros::format_description;
use url::Url;

use crate::client::Client;
use crate::enums::LabelQuadrant;
use crate::error::{
    Error, HeldBackShipments, PaymentInstruction, Result, ShipmentCharge, ValidationError,
};
use crate::http::{Context, Host, Operation, Request, Response};
use crate::types::{ShipmentIds, join_ids};

const PDF_ACCEPT: &str = "application/pdf";
const LINK_ACCEPT: &str = "application/vnd.shipment_label_link+json";

/// Where labels are placed on the first A4 sheet.
///
/// Later sheets always use the default positions `1;2;3;4`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A4Positions {
    /// Starts at a quadrant and fills the quadrants after it.
    StartingAt(LabelQuadrant),
    /// Uses exactly these quadrants.
    Exact(Vec<LabelQuadrant>),
}

impl A4Positions {
    /// Uses exactly these quadrants: one to four, each at most once.
    pub fn exact(quadrants: Vec<LabelQuadrant>) -> Result<Self, ValidationError> {
        if quadrants.is_empty() || quadrants.len() > 4 {
            return Err(ValidationError::new(
                "label positions",
                "one to four positions are required",
            ));
        }
        let mut seen = quadrants.clone();
        seen.sort_unstable();
        seen.dedup();
        if seen.len() != quadrants.len() {
            return Err(ValidationError::new(
                "label positions",
                "positions must be unique",
            ));
        }
        Ok(Self::Exact(quadrants))
    }

    fn encode(&self) -> String {
        let join = |values: &mut dyn Iterator<Item = LabelQuadrant>| {
            values
                .map(|q| q.value().to_string())
                .collect::<Vec<_>>()
                .join(";")
        };
        match self {
            Self::StartingAt(first) => {
                join(&mut LabelQuadrant::ALL.iter().copied().filter(|q| q >= first))
            }
            Self::Exact(quadrants) => join(&mut quadrants.iter().copied()),
        }
    }
}

/// The paper format of a label PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelFormat {
    /// One label on an A6 sheet.
    A6,
    /// Labels on an A4 sheet.
    A4(A4Positions),
}

impl Default for LabelFormat {
    /// A4, starting at the top left.
    fn default() -> Self {
        Self::A4(A4Positions::StartingAt(LabelQuadrant::TopLeftQuadrant))
    }
}

impl LabelFormat {
    pub(crate) fn query(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::A6 => vec![("format", "A6".to_owned())],
            Self::A4(positions) => vec![
                ("format", "A4".to_owned()),
                ("positions", positions.encode()),
            ],
        }
    }
}

/// A label PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelPdf {
    /// The PDF.
    pub bytes: Vec<u8>,
    /// A file name for saving the PDF: `myparcel-label-` followed by the time it was fetched.
    pub filename: String,
}

fn filename() -> String {
    let format = format_description!("[year]-[month repr:short]-[day] [hour]-[minute]-[second]");
    let stamp = OffsetDateTime::now_utc()
        .format(format)
        .expect("a timestamp always formats");
    format!("myparcel-label-{stamp}.pdf")
}

fn payment_required(body: &[u8]) -> Option<Error> {
    #[derive(Deserialize)]
    struct Envelope<T> {
        data: T,
    }
    #[derive(Deserialize)]
    struct Instructions {
        payment_instructions: Vec<PaymentInstruction>,
    }
    #[derive(Deserialize)]
    struct Shipments {
        shipments: HeldBack,
    }
    #[derive(Deserialize)]
    struct HeldBack {
        unpaid_shipments: Vec<ShipmentCharge>,
        blocked_shipments: Vec<ShipmentCharge>,
    }

    if let Ok(envelope) = serde_json::from_slice::<Envelope<Instructions>>(body) {
        return Some(Error::PaymentRequired(envelope.data.payment_instructions));
    }
    serde_json::from_slice::<Envelope<Shipments>>(body)
        .ok()
        .map(|envelope| {
            Error::LabelsHeldBack(HeldBackShipments {
                unpaid: envelope.data.shipments.unpaid_shipments,
                blocked: envelope.data.shipments.blocked_shipments,
            })
        })
}

fn label_request(ids: &ShipmentIds, format: &LabelFormat, accept: &'static str) -> Request {
    let mut request = Request::new(
        Method::GET,
        Host::Core,
        vec!["shipment_labels".into(), join_ids(ids)],
        accept,
    );
    for (name, value) in format.query() {
        request = request.query(name, value);
    }
    request
}

pub(crate) struct Pdf {
    ids: ShipmentIds,
    format: LabelFormat,
}

impl Pdf {
    pub fn new(ids: &ShipmentIds, format: &LabelFormat) -> Self {
        Self {
            ids: ids.clone(),
            format: format.clone(),
        }
    }
}

impl Operation for Pdf {
    type Output = LabelPdf;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Ok(label_request(&self.ids, &self.format, PDF_ACCEPT))
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        if response.status == 402 {
            if let Some(error) = payment_required(&response.body) {
                return Err(error);
            }
        }
        let response = response.ensure_success()?;
        let is_pdf = response.body.starts_with(b"%PDF-")
            && response.body.get(5).is_some_and(u8::is_ascii_digit);
        if is_pdf {
            return Ok(LabelPdf {
                bytes: response.body,
                filename: filename(),
            });
        }
        Err(payment_required(&response.body)
            .unwrap_or(Error::UnexpectedResponse("the response is not a pdf")))
    }
}

pub(crate) struct Link {
    ids: ShipmentIds,
    format: LabelFormat,
}

impl Link {
    pub fn new(ids: &ShipmentIds, format: &LabelFormat) -> Self {
        Self {
            ids: ids.clone(),
            format: format.clone(),
        }
    }
}

impl Operation for Link {
    type Output = Url;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Ok(label_request(&self.ids, &self.format, LINK_ACCEPT))
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        #[derive(Deserialize)]
        struct Envelope {
            data: Data,
        }
        #[derive(Deserialize)]
        struct Data {
            pdfs: Option<Target>,
            pdf: Option<Target>,
        }
        #[derive(Deserialize)]
        struct Target {
            url: String,
        }

        if response.status == 402 {
            if let Some(error) = payment_required(&response.body) {
                return Err(error);
            }
        }
        let response = response.ensure_success()?;
        let envelope: Envelope = response.json()?;
        let target = envelope
            .data
            .pdfs
            .or(envelope.data.pdf)
            .ok_or(Error::UnexpectedResponse("the response has no label link"))?;
        response
            .url
            .join(&target.url)
            .map_err(|_| Error::UnexpectedResponse("the label link is not a valid url"))
    }
}

macro_rules! labels_methods {
    ($($async:ident)?; $($await:ident)?) => {
        /// Downloads the labels of shipments as one PDF.
        ///
        /// Fails with [`Error::PaymentRequired`](crate::Error::PaymentRequired) when the account
        /// has to pay first.
        pub $($async)? fn pdf(
            &self,
            ids: &$crate::types::ShipmentIds,
            format: &$crate::labels::LabelFormat,
        ) -> $crate::error::Result<$crate::labels::LabelPdf> {
            self.client.execute($crate::labels::Pdf::new(ids, format)) $(.$await)?
        }

        /// Returns a link to the labels of shipments.
        pub $($async)? fn link(
            &self,
            ids: &$crate::types::ShipmentIds,
            format: &$crate::labels::LabelFormat,
        ) -> $crate::error::Result<::url::Url> {
            self.client.execute($crate::labels::Link::new(ids, format)) $(.$await)?
        }
    };
}

#[cfg(feature = "blocking")]
pub(crate) use labels_methods;

/// Operations on labels.
#[derive(Debug, Clone, Copy)]
pub struct Labels<'a> {
    pub(crate) client: &'a Client,
}

impl Labels<'_> {
    labels_methods!(async; await);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_at_fills_the_following_quadrants() {
        let format = LabelFormat::A4(A4Positions::StartingAt(LabelQuadrant::TopRightQuadrant));
        assert_eq!(
            format.query(),
            vec![("format", "A4".into()), ("positions", "2;3;4".into())]
        );
        assert_eq!(LabelFormat::default().query()[1].1, "1;2;3;4");
    }

    #[test]
    fn exact_positions_keep_their_order() {
        let positions = A4Positions::exact(vec![
            LabelQuadrant::TopLeftQuadrant,
            LabelQuadrant::BottomRightQuadrant,
        ])
        .unwrap();
        assert_eq!(LabelFormat::A4(positions).query()[1].1, "1;4");
        assert!(A4Positions::exact(vec![]).is_err());
        assert!(A4Positions::exact(vec![LabelQuadrant::TopLeftQuadrant; 2]).is_err());
    }

    #[test]
    fn a6_has_no_positions() {
        assert_eq!(LabelFormat::A6.query(), vec![("format", "A6".to_owned())]);
    }
}
