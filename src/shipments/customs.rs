use serde::Serialize;

use crate::country::CountryCode;
use crate::enums::CustomsContents;
use crate::error::ValidationError;
use crate::types::{Grams, Money, NonEmpty};

bounded_string! {
    /// An invoice number of at most 150 characters.
    InvoiceNumber, "invoice", 1, 150
}

bounded_string! {
    /// A product reference of at most 64 characters.
    ExternalProductReference, "external product reference", 1, 64
}

/// The number of units of a customs item, 1 to 99 999.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ItemQuantity(u32);

impl ItemQuantity {
    /// Validates a quantity.
    pub fn new(quantity: u32) -> Result<Self, ValidationError> {
        if !(1..=99_999).contains(&quantity) {
            return Err(ValidationError::new(
                "item quantity",
                format!("{quantity} is outside 1..=99999"),
            ));
        }
        Ok(Self(quantity))
    }

    /// The quantity.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// An item listed in a customs declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CustomsItem {
    description: String,
    amount: ItemQuantity,
    weight: Grams,
    item_value: Money,
    classification: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    country: Option<CountryCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_product_reference: Option<ExternalProductReference>,
}

impl CustomsItem {
    /// An item with the fields the API requires.
    ///
    /// `quantity` is the number of units, `weight` the weight of all units together and
    /// `classification` the HS code.
    pub fn new(
        description: impl Into<String>,
        quantity: ItemQuantity,
        weight: Grams,
        value: Money,
        classification: impl Into<String>,
    ) -> Result<Self, ValidationError> {
        let (description, classification) = (description.into(), classification.into());
        if description.is_empty() {
            return Err(ValidationError::new(
                "item description",
                "must not be empty",
            ));
        }
        if classification.is_empty() {
            return Err(ValidationError::new(
                "item classification",
                "must not be empty",
            ));
        }
        Ok(Self {
            description,
            amount: quantity,
            weight,
            item_value: value,
            classification,
            country: None,
            external_product_reference: None,
        })
    }

    /// Sets the country of origin.
    pub fn country(mut self, value: CountryCode) -> Self {
        self.country = Some(value);
        self
    }

    /// Sets your own product reference.
    pub fn external_product_reference(mut self, value: ExternalProductReference) -> Self {
        self.external_product_reference = Some(value);
        self
    }
}

/// The customs declaration of an international shipment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CustomsDeclaration {
    contents: CustomsContents,
    weight: Grams,
    items: NonEmpty<CustomsItem, 100>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invoice: Option<InvoiceNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eori_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vat_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vat_number_uk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    voec_number: Option<String>,
}

impl CustomsDeclaration {
    /// A declaration of what is shipped, its total weight and the items.
    pub fn new(
        contents: CustomsContents,
        weight: Grams,
        items: NonEmpty<CustomsItem, 100>,
    ) -> Self {
        Self {
            contents,
            weight,
            items,
            invoice: None,
            eori_number: None,
            vat_number: None,
            vat_number_uk: None,
            voec_number: None,
        }
    }

    /// Sets the invoice number.
    pub fn invoice(mut self, value: InvoiceNumber) -> Self {
        self.invoice = Some(value);
        self
    }

    /// Sets the EORI number.
    pub fn eori_number(mut self, value: impl Into<String>) -> Self {
        self.eori_number = Some(value.into());
        self
    }

    /// Sets the VAT number.
    pub fn vat_number(mut self, value: impl Into<String>) -> Self {
        self.vat_number = Some(value.into());
        self
    }

    /// Sets the UK VAT number.
    pub fn vat_number_uk(mut self, value: impl Into<String>) -> Self {
        self.vat_number_uk = Some(value.into());
        self
    }

    /// Sets the VOEC number.
    pub fn voec_number(mut self, value: impl Into<String>) -> Self {
        self.voec_number = Some(value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Cents;

    #[test]
    fn quantity_bounds() {
        assert!(ItemQuantity::new(0).is_err());
        assert!(ItemQuantity::new(1).is_ok());
        assert!(ItemQuantity::new(99_999).is_ok());
        assert!(ItemQuantity::new(100_000).is_err());
    }

    #[test]
    fn serializes_item_value_as_money() {
        let item = CustomsItem::new(
            "Socks",
            ItemQuantity::new(2).unwrap(),
            Grams::new(300).unwrap(),
            Money::eur(Cents::new(1250)),
            "611595",
        )
        .unwrap()
        .country(CountryCode::NL);
        assert_eq!(
            serde_json::to_string(&item).unwrap(),
            r#"{"description":"Socks","amount":2,"weight":300,"item_value":{"amount":1250,"currency":"EUR"},"classification":"611595","country":"NL"}"#
        );
    }
}
