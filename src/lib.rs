//! Unofficial client for the [MyParcel](https://www.myparcel.nl) API.
//!
//! ```no_run
//! use myparcel_rs::country::CountryCode;
//! use myparcel_rs::enums::{Carrier, PackageType};
//! use myparcel_rs::labels::LabelFormat;
//! use myparcel_rs::shipments::{
//!     City, HouseNumber, NewShipment, Person, PostalCode, Recipient, ShipmentBatch, ShipmentOptions,
//!     Street,
//! };
//! use myparcel_rs::types::ShipmentIds;
//! use myparcel_rs::{ApiKey, Client};
//!
//! # async fn run() -> myparcel_rs::Result<()> {
//! let client = Client::new(ApiKey::new(std::env::var("MYPARCEL_API_KEY").unwrap())?)?;
//!
//! let recipient = Recipient::new(
//!     CountryCode::NL,
//!     PostalCode::new("2132 JE")?,
//!     City::new("Hoofddorp")?,
//!     Street::new("Antareslaan")?,
//!     Person::new("John Doe")?,
//! )
//! .number(HouseNumber::new("31")?);
//! let shipment = NewShipment::new(Carrier::PostNl, recipient, ShipmentOptions::new(PackageType::Package));
//!
//! let created = client.shipments().create(&ShipmentBatch::one(shipment), None).await?;
//! let ids = ShipmentIds::new(created.shipments.iter().map(|s| s.id).collect())?;
//! let pdf = client.labels().pdf(&ids, &LabelFormat::default()).await?;
//! std::fs::write(&pdf.filename, &pdf.bytes).unwrap();
//! # Ok(())
//! # }
//! ```
//!
//! Everything you send is validated when you construct it, so a value the API would refuse never
//! reaches the network. Values the API reports are kept as they are: an enum value this crate does
//! not know is returned as [`Reported::Unknown`] instead of failing the response.
//!
//! # Features
//!
//! - `rustls-tls` (default): TLS through rustls.
//! - `native-tls`: TLS through the platform library.
//! - `blocking`: the [`blocking`] client with the same operations.
//! - `connect`: [`connect`], MyParcel Connect with proof of possession and order push.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[macro_use]
mod macros;

mod auth;
mod client;
mod error;
mod http;
mod reported;
mod serde_util;

#[cfg(feature = "blocking")]
pub mod blocking;

pub mod account;
pub mod capabilities;
#[cfg(feature = "connect")]
pub mod connect;
pub mod country;
pub mod enums;
pub mod labels;
pub mod returns;
pub mod shipments;
pub mod track_trace;
pub mod types;
pub mod webhooks;

pub use auth::ApiKey;
pub use client::{Client, ClientBuilder};
pub use error::{
    ApiError, ApiErrorDetail, Error, HeldBackShipments, PaymentInstruction, Result, ShipmentCharge,
    ValidationError,
};
pub use reported::Reported;
