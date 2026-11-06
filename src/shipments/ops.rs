use reqwest::Method;
use serde::Deserialize;
use serde_json::{Value, json};
use url::Url;

use crate::error::{Error, Result};
use crate::http::{Context, Host, Operation, Request, Response};
use crate::labels::LabelFormat;
use crate::serde_util::opt_string_or_int;
use crate::shipments::ShipmentBatch;
use crate::shipments::new::generated_reference;
use crate::shipments::query::ShipmentQuery;
use crate::shipments::response::Shipment;
use crate::types::{PrinterGroupId, ShipmentId, ShipmentIds, join_ids};

const SHIPMENT_CONTENT_TYPE: &str = "application/vnd.shipment+json;version=1.1";
const CREATE_ACCEPT: &str = "application/json;charset=utf-8;version=1.2,application/json,application/vnd.shipment_label+json;q=0.9,application/problem+json;q=0.9,application/*;q=0.8";
const JSON_ACCEPT: &str = "application/json";
const DELETE_ACCEPT: &str = "application/json,application/problem+json;q=0.9";

/// A shipment that was created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedShipment {
    /// The id of the shipment.
    pub id: ShipmentId,
    /// The reference the shipment was created with.
    pub reference: Option<String>,
}

/// The result of creating shipments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedShipments {
    /// The created shipments, in the order they were sent.
    pub shipments: Vec<CreatedShipment>,
    /// A link to the label PDF, when the API created labels right away.
    pub label_url: Option<Url>,
}

struct Prepared {
    shipments: Vec<Value>,
    references: Vec<String>,
}

fn prepare(batch: &ShipmentBatch) -> Result<Prepared> {
    let mut shipments = Vec::with_capacity(batch.len());
    let mut references = Vec::with_capacity(batch.len());
    for shipment in batch {
        let mut value = serde_json::to_value(shipment).map_err(|source| Error::Decode {
            source,
            body: String::new(),
        })?;
        let reference = match shipment.reference() {
            Some(reference) => reference.clone(),
            None => generated_reference("sdk_")?,
        };
        value["reference_identifier"] = Value::String(reference.as_str().to_owned());
        references.push(reference.to_string());
        shipments.push(value);
    }
    Ok(Prepared {
        shipments,
        references,
    })
}

fn body(prepared: &Prepared, cx: &Context<'_>) -> Value {
    json!({"data": {"user_agent": cx.user_agent, "shipments": prepared.shipments}})
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}

#[derive(Deserialize)]
struct Item {
    id: ShipmentId,
    #[serde(default, deserialize_with = "opt_string_or_int")]
    reference_identifier: Option<String>,
}

fn created(items: Vec<Item>, requested: &[String]) -> Vec<CreatedShipment> {
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| CreatedShipment {
            id: item.id,
            reference: item
                .reference_identifier
                .or_else(|| requested.get(index).cloned()),
        })
        .collect()
}

pub(crate) struct Create {
    prepared: Prepared,
    format: Option<LabelFormat>,
}

impl Create {
    pub fn new(batch: &ShipmentBatch, format: Option<&LabelFormat>) -> Result<Self> {
        Ok(Self {
            prepared: prepare(batch)?,
            format: format.cloned(),
        })
    }
}

impl Operation for Create {
    type Output = CreatedShipments;

    fn request(&self, cx: &Context<'_>) -> Result<Request> {
        let mut request = Request::new(
            Method::POST,
            Host::Core,
            vec!["shipments".into()],
            CREATE_ACCEPT,
        );
        if let Some(format) = &self.format {
            for (name, value) in format.query() {
                request = request.query(name, value);
            }
        }
        request.json_body(SHIPMENT_CONTENT_TYPE, &body(&self.prepared, cx))
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        #[derive(Deserialize)]
        struct Data {
            #[serde(alias = "ids")]
            shipments: Vec<Item>,
            pdf: Option<Link>,
        }
        #[derive(Deserialize)]
        struct Link {
            url: String,
        }

        let response = response.ensure_success()?;
        let data = response.json::<Envelope<Data>>()?.data;
        let label_url = data
            .pdf
            .map(|link| response.url.join(&link.url))
            .transpose()
            .map_err(|_| Error::UnexpectedResponse("the label link is not a valid url"))?;
        Ok(CreatedShipments {
            shipments: created(data.shipments, &self.prepared.references),
            label_url,
        })
    }
}

pub(crate) struct Print {
    prepared: Prepared,
    printer_group: String,
}

impl Print {
    pub fn new(batch: &ShipmentBatch, printer_group: &PrinterGroupId) -> Result<Self> {
        Ok(Self {
            prepared: prepare(batch)?,
            printer_group: printer_group.to_string(),
        })
    }
}

impl Operation for Print {
    type Output = Vec<CreatedShipment>;

    fn request(&self, cx: &Context<'_>) -> Result<Request> {
        let accept = format!(
            "application/vnd.shipment_label+json+print;printer-group-id={}",
            self.printer_group
        );
        Request::new(Method::POST, Host::Core, vec!["shipments".into()], accept)
            .json_body(SHIPMENT_CONTENT_TYPE, &body(&self.prepared, cx))
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        #[derive(Deserialize)]
        struct Data {
            ids: Vec<Item>,
        }

        let data = response.ensure_success()?.json::<Envelope<Data>>()?.data;
        Ok(created(data.ids, &self.prepared.references))
    }
}

pub(crate) struct Query {
    params: Vec<(&'static str, String)>,
}

impl Query {
    pub fn new(query: ShipmentQuery) -> Self {
        Self {
            params: query.into_params(),
        }
    }
}

fn parse_shipments(response: Response) -> Result<Vec<Shipment>> {
    #[derive(Deserialize)]
    struct Data {
        shipments: Vec<Shipment>,
    }

    Ok(response
        .ensure_success()?
        .json::<Envelope<Data>>()?
        .data
        .shipments)
}

impl Operation for Query {
    type Output = Vec<Shipment>;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        let mut request = Request::new(
            Method::GET,
            Host::Core,
            vec!["shipments".into()],
            JSON_ACCEPT,
        );
        for (name, value) in &self.params {
            request = request.query(name, value.clone());
        }
        Ok(request)
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        parse_shipments(response)
    }
}

pub(crate) struct Get {
    ids: String,
}

impl Get {
    pub fn new(ids: &ShipmentIds) -> Self {
        Self { ids: join_ids(ids) }
    }
}

impl Operation for Get {
    type Output = Vec<Shipment>;

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Ok(Request::new(
            Method::GET,
            Host::Core,
            vec!["shipments".into(), self.ids.clone()],
            JSON_ACCEPT,
        ))
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        parse_shipments(response)
    }
}

pub(crate) struct Delete {
    ids: String,
}

impl Delete {
    pub fn new(ids: &ShipmentIds) -> Self {
        Self { ids: join_ids(ids) }
    }
}

impl Operation for Delete {
    type Output = ();

    fn request(&self, _: &Context<'_>) -> Result<Request> {
        Ok(Request::new(
            Method::DELETE,
            Host::Core,
            vec!["shipments".into(), self.ids.clone()],
            DELETE_ACCEPT,
        ))
    }

    fn parse(&self, response: Response) -> Result<Self::Output> {
        response.ensure_success().map(drop)
    }
}
