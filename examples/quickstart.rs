use myparcel_rs::country::CountryCode;
use myparcel_rs::enums::{Carrier, PackageType};
use myparcel_rs::labels::LabelFormat;
use myparcel_rs::shipments::{
    City, HouseNumber, NewShipment, Person, PostalCode, Recipient, ShipmentBatch, ShipmentOptions,
    Street,
};
use myparcel_rs::types::ShipmentIds;
use myparcel_rs::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?;

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

    let created = client
        .shipments()
        .create(&ShipmentBatch::one(shipment), None)
        .await?;
    let ids = ShipmentIds::new(
        created
            .shipments
            .iter()
            .map(|shipment| shipment.id)
            .collect(),
    )?;
    println!("created {} concept(s)", ids.len());

    let pdf = client.labels().pdf(&ids, &LabelFormat::default()).await?;
    std::fs::write(&pdf.filename, &pdf.bytes).expect("the label can be written");
    println!("label saved as {}", pdf.filename);

    let traces = client.track_traces().fetch(&ids).await?;
    for (id, trace) in traces {
        println!("{id}: {}", trace.description);
    }

    client.shipments().delete(&ids).await?;
    Ok(())
}
