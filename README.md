# myparcel-rs

Async Rust client for the [MyParcel](https://www.myparcel.nl) API.

> This is an unofficial, community-maintained library. It is not affiliated with, endorsed by or supported by MyParcel.

It covers shipments, labels, track and trace, return shipments, multi-collo, webhooks, carrier capabilities and contract definitions, the account behind an API key and, behind a feature, MyParcel Connect with order push.

```toml
[dependencies]
myparcel-rs = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The full API documentation is on [docs.rs](https://docs.rs/myparcel-rs).

## Usage

```rust
use myparcel_rs::country::CountryCode;
use myparcel_rs::enums::{Carrier, PackageType};
use myparcel_rs::labels::LabelFormat;
use myparcel_rs::shipments::{
    City, HouseNumber, NewShipment, Person, PostalCode, Recipient, ShipmentBatch, ShipmentOptions,
    Street,
};
use myparcel_rs::types::ShipmentIds;
use myparcel_rs::{ApiKey, Client};

#[tokio::main]
async fn main() -> myparcel_rs::Result<()> {
    let client = Client::new(ApiKey::new(std::env::var("MYPARCEL_API_KEY").unwrap())?)?;

    let recipient = Recipient::new(
        CountryCode::NL,
        PostalCode::new("2132 JE")?,
        City::new("Hoofddorp")?,
        Street::new("Antareslaan")?,
        Person::new("John Doe")?,
    )
    .number(HouseNumber::new("31")?);
    let shipment = NewShipment::new(
        Carrier::PostNl,
        recipient,
        ShipmentOptions::new(PackageType::Package),
    );

    let created = client.shipments().create(&ShipmentBatch::one(shipment), None).await?;
    let ids = ShipmentIds::new(created.shipments.iter().map(|s| s.id).collect())?;

    let pdf = client.labels().pdf(&ids, &LabelFormat::default()).await?;
    std::fs::write(&pdf.filename, &pdf.bytes).unwrap();
    Ok(())
}
```

Create an API key in the MyParcel backoffice. `Client::from_env()` reads `API_KEY`, then `API_KEY_NL`, then `API_KEY_BE`.

### Types instead of strings

What you send is validated when you construct it, with the limits of the API's specification: a `Street` is at most 40 characters and not only digits, a `LabelDescription` at most 45, a `ReferenceIdentifier` has no `=` or `?`, an `Insurance` is at most 25 000 euros, a `CountryCode` is an ISO 3166-1 alpha-2 code. Lists that must not be empty are `NonEmpty`, with the maximum of the API where there is one: a `ShipmentBatch` holds 1 to 100 shipments. Nothing is truncated, dropped or replaced by a default on the way: a value that does not fit is an error.

Which options a carrier allows for a package type and destination is decided by the API. Ask the capabilities service before you offer them.

What the API reports is read more leniently. MyParcel extends its enums regularly, so a carrier, status or hook this crate does not know is returned as `Reported::Unknown` with the raw value instead of failing the whole response:

```rust
match shipment.status {
    Some(Reported::Known(status)) => println!("{status:?}"),
    Some(Reported::Unknown(raw)) => println!("a status this version does not know: {raw}"),
    None => {}
}
```

### Labels

`LabelFormat` is `A6` or `A4` with positions: `StartingAt(quadrant)` fills the quadrants from there on, `Exact` uses exactly the quadrants you give. `labels().pdf` returns the bytes and a file name; `labels().link` returns a link. Labels the account has to pay for are reported as `Error::PaymentRequired` with the payment instructions, and labels the API holds back as `Error::LabelsHeldBack`, which lists the unpaid and the blocked shipments with their prices.

### Multi-collo

`NewShipment::split_into(colli)` turns a shipment into a multi-collo shipment: the weight is divided over the colli (rounded down), and all colli share the reference.

### Returns

`returns().create_related` creates a return for an existing shipment, optionally emailing the label to the consumer. `returns().create_unrelated` creates a return without a parent.

### Webhooks

```rust
let url: WebhookUrl = "https://example.com/webhook".parse()?;
let id = client.webhooks().subscribe(Hook::ShipmentStatusChange, &url).await?;
```

There is one subscription per hook; subscribing again replaces it. Keep the id, you need it to unsubscribe. MyParcel only delivers to `https` urls.

### Capabilities

```rust
let request = CapabilitiesRequest::for_country(CountryCode::NL)
    .carrier(CarrierV2::PostNl)
    .package_type(PackageType::Package);
let answer = client.capabilities().get(&request).await?;
```

`capabilities().contract_definitions` lists what each carrier offers across the contracts of the account.

### Errors

Failed requests return `Error::Api` with the status, the message, the request id and the problems the API lists:

```rust
match client.shipments().delete(&ids).await {
    Err(e) if e.is_unauthorized() => println!("the api key was refused"),
    Err(e) => return Err(e),
    Ok(()) => {}
}
```

The body of an error response is cut to 500 characters. It can contain personal data, so think twice before logging it.

### Timeouts, redirects and TLS

Requests time out after 60 seconds (10 seconds to connect); change it with `Client::builder(key).timeout(..)`. The default HTTP client does not follow redirects: the API is not expected to send any, and following one could carry your key to another host. If you pass your own `reqwest::Client`, its settings apply as is.

Base urls must use `https`. Plain `http` is only accepted for loopback addresses, which is useful for mock servers. The API key is never shown by `Debug` and sent as a sensitive header. The crate's own copy is wiped from memory when dropped; the copy in a request header, held by the HTTP client while the request runs, is not.

Nothing is retried automatically: sending a create again could create a second shipment.

### Identifying your integration

`Client::builder(key).user_agent("WooCommerce", "9.1")` puts `WooCommerce/9.1` in front of the crate's own user agent. MyParcel asks integrations of known platforms to identify themselves.

### Blocking client

With the `blocking` feature, `myparcel_rs::blocking::Client` has the same operations without `async`. Build it with `Client::builder(key).build_blocking()` or `blocking::Client::new(key)`. Do not use it from inside an async runtime. A client passed to `http_client` is ignored by the blocking client.

### MyParcel Connect

With the `connect` feature, `ConnectService` connects a shop through MyParcel Connect and pushes its orders. Shopify is the only platform MyParcel has enabled, and the account needs order management. The merchant logs in at MyParcel and is redirected back to your shop; every request after that is signed with a key that never leaves the shop (DPoP, ES256), so a stolen token is useless on its own.

```rust
let config = ConnectConfig::new(ConnectPlatform::Shopify, EncryptionKey::new(secret)?);
let connect = ConnectService::new(config, storage)?;

// your start route
let url = connect.start(&ShopName::new("My shop")?, &"https://shop.example".parse()?, None).await?;

// your callback route, at CALLBACK_PATH
connect.handle_callback(&CallbackQuery::from_query_string(query)).await?;

let results = connect.push_orders(&orders).await?;
```

You provide the storage by implementing `ConnectStorage` (or `BlockingConnectStorage`). The private key and the access token are encrypted with your `EncryptionKey` before they reach it; the rest of the records is stored as it is. Keep storage out of web-served paths and logs and treat backups as credentials.

Connect covers pushing orders. Everything else needs an API key. There is no revoke endpoint: `disconnect` and `uninstall` only forget the connection locally, and the old token stays valid until it expires.

## Features

| Feature | Default | |
|---|---|---|
| `rustls-tls` | yes | TLS through rustls |
| `native-tls` | no | TLS through the platform library |
| `blocking` | no | The blocking client |
| `connect` | no | MyParcel Connect and order push |

Use `default-features = false, features = ["native-tls"]` to switch TLS backends. The minimum supported Rust version is 1.87.

## What is not included

Not covered: fulfilment v1 orders and notes, printer groups, and the delivery options, pick-up locations, drop-off points and rates endpoints.

Some rarely used fields of the shipment request are not exposed: `status`, `delivered`, `recipients`, `shipped_items` and `collection_contact`.

## Testing

```sh
cargo test --all-features
```

The tests run against a mock server. Next to them, the specification of MyParcel is part of the repository. The tests check that the main enums (carriers, package and delivery types, statuses, label positions and more) have exactly the values of the specification, that the requests this crate sends (shipments, returns, webhook subscriptions, capabilities, contract definitions and Connect orders) satisfy their schemas, and that the response fixtures do the same. To exercise the real API with a throwaway concept shipment:

```sh
MYPARCEL_API_KEY=... cargo test --test live -- --ignored
```

## Supported API versions

| | Version |
|---|---|
| Core API specification | `2026-04-16`, from `https://api.myparcel.nl/openapi.min.json` |
| MyParcel Connect specification | 1.0.0 |

| Endpoint | Version used |
|---|---|
| `POST /shipments` | Request `application/vnd.shipment+json;version=1.1`, response `version=1.2` |
| Return shipments | `application/vnd.return_shipment+json`, `application/vnd.unrelated_return_shipment+json` |
| `GET /shipments`, `GET`/`DELETE /shipments/{ids}` | Unversioned |
| `GET /shipment_labels/{ids}` | `application/pdf`, `application/vnd.shipment_label_link+json` |
| `GET /tracktraces/{ids}` | Unversioned |
| `/webhook_subscriptions` | `version=1.1` |
| `POST /shipments/capabilities`, `/shipments/capabilities/contract-definitions` | `version=2.0` |
| `GET /whoami` (IAM, `https://iam.api.myparcel.nl`) | Its specification is not public, so the fields are not checked against one |
| Connect: `/connect/start`, `/connect/token`, `/connect/refresh`, `/webhook/orders` | Specification 1.0.0 |

## License

MIT

---

This library is developed by [Solvalutions](https://solvalutions.nl) and [Rob Meijerink](https://robmeijerink.nl).

*&copy;&nbsp;2026 &mdash; Built by Rob Meijerink.*
