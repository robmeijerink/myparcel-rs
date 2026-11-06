use serde::Serialize;

use crate::country::CountryCode;
use crate::error::ValidationError;
use crate::types::NonEmpty;

bounded_string! {
    /// A postal code of at most 10 characters.
    PostalCode, "postal code", 1, 10
}

bounded_string! {
    /// A city of 1 to 35 characters.
    City, "city", 1, 35
}

bounded_string! {
    /// A street of 1 to 40 characters that is not made of digits only.
    Street, "street", 1, 40,
    |value| value.chars().all(|c| c.is_ascii_digit()).then_some("must contain a character that is not a digit")
}

bounded_string! {
    /// Additional street information of at most 50 characters.
    StreetAdditionalInfo, "street additional info", 1, 50
}

bounded_string! {
    /// A house number of at most 8 characters.
    HouseNumber, "house number", 1, 8
}

bounded_string! {
    /// A house number suffix of at most 6 characters.
    NumberSuffix, "number suffix", 1, 6
}

bounded_string! {
    /// A box number of at most 8 characters.
    BoxNumber, "box number", 1, 8
}

bounded_string! {
    /// A region of at most 35 characters.
    Region, "region", 1, 35
}

bounded_string! {
    /// A state of at most 2 characters.
    State, "state", 1, 2
}

bounded_string! {
    /// An area of at most 35 characters.
    Area, "area", 1, 35
}

bounded_string! {
    /// The name of a person, 1 to 50 characters.
    Person, "person", 1, 50
}

bounded_string! {
    /// The name of a company of at most 50 characters.
    Company, "company", 1, 50
}

bounded_string! {
    /// A phone number of at most 25 characters.
    Phone, "phone", 1, 25
}

bounded_string! {
    /// An email address.
    ///
    /// Only the shape `local@domain` is checked: both parts must be present, nothing may contain
    /// whitespace or control characters and the address is at most 254 characters long.
    EmailAddress, "email address", 3, 254,
    |value| {
        if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Some("must not contain whitespace or control characters");
        }
        match value.rsplit_once('@') {
            Some((local, domain)) if !local.is_empty() && !domain.is_empty() => None,
            _ => Some("must look like local@domain"),
        }
    }
}

/// Who a shipment is delivered to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recipient {
    cc: CountryCode,
    postal_code: PostalCode,
    city: City,
    street: Street,
    person: Person,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<State>,
    #[serde(skip_serializing_if = "Option::is_none")]
    street_additional_info: Option<StreetAdditionalInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<HouseNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    number_suffix: Option<NumberSuffix>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box_number: Option<BoxNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    area: Option<Area>,
    #[serde(skip_serializing_if = "Option::is_none")]
    company: Option<Company>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<EmailAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secondary_emails: Option<NonEmpty<EmailAddress, 4>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<Phone>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eori_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vat_number: Option<String>,
}

impl Recipient {
    /// A recipient with the fields the API requires.
    pub fn new(
        cc: CountryCode,
        postal_code: PostalCode,
        city: City,
        street: Street,
        person: Person,
    ) -> Self {
        Self {
            cc,
            postal_code,
            city,
            street,
            person,
            region: None,
            state: None,
            street_additional_info: None,
            number: None,
            number_suffix: None,
            box_number: None,
            area: None,
            company: None,
            email: None,
            secondary_emails: None,
            phone: None,
            eori_number: None,
            vat_number: None,
        }
    }

    /// Sets the region.
    pub fn region(mut self, value: Region) -> Self {
        self.region = Some(value);
        self
    }

    /// Sets the state.
    pub fn state(mut self, value: State) -> Self {
        self.state = Some(value);
        self
    }

    /// Sets additional street information.
    pub fn street_additional_info(mut self, value: StreetAdditionalInfo) -> Self {
        self.street_additional_info = Some(value);
        self
    }

    /// Sets the house number.
    pub fn number(mut self, value: HouseNumber) -> Self {
        self.number = Some(value);
        self
    }

    /// Sets the house number suffix.
    pub fn number_suffix(mut self, value: NumberSuffix) -> Self {
        self.number_suffix = Some(value);
        self
    }

    /// Sets the box number.
    pub fn box_number(mut self, value: BoxNumber) -> Self {
        self.box_number = Some(value);
        self
    }

    /// Sets the area.
    pub fn area(mut self, value: Area) -> Self {
        self.area = Some(value);
        self
    }

    /// Sets the company.
    pub fn company(mut self, value: Company) -> Self {
        self.company = Some(value);
        self
    }

    /// Sets the email address.
    pub fn email(mut self, value: EmailAddress) -> Self {
        self.email = Some(value);
        self
    }

    /// Sets up to four additional email addresses.
    pub fn secondary_emails(mut self, value: NonEmpty<EmailAddress, 4>) -> Self {
        self.secondary_emails = Some(value);
        self
    }

    /// Sets the phone number.
    pub fn phone(mut self, value: Phone) -> Self {
        self.phone = Some(value);
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

    /// The country of the recipient.
    pub fn country(&self) -> CountryCode {
        self.cc
    }
}

/// A sender that differs from the one configured in the shop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sender {
    cc: CountryCode,
    postal_code: String,
    city: String,
    street: String,
    person: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    street_additional_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    number_suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box_number: Option<BoxNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
    #[serde(skip_serializing_if = "Option::is_none")]
    company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<EmailAddress>,
}

impl Sender {
    /// A sender with the fields the API requires. None of them may be empty.
    pub fn new(
        cc: CountryCode,
        postal_code: impl Into<String>,
        city: impl Into<String>,
        street: impl Into<String>,
        person: impl Into<String>,
    ) -> Result<Self, ValidationError> {
        let (postal_code, city, street, person) = (
            postal_code.into(),
            city.into(),
            street.into(),
            person.into(),
        );
        for (field, value) in [
            ("sender postal code", &postal_code),
            ("sender city", &city),
            ("sender street", &street),
            ("sender person", &person),
        ] {
            if value.is_empty() {
                return Err(ValidationError::new(field, "must not be empty"));
            }
        }
        Ok(Self {
            cc,
            postal_code,
            city,
            street,
            person,
            street_additional_info: None,
            number: None,
            number_suffix: None,
            box_number: None,
            region: None,
            company: None,
            phone: None,
            email: None,
        })
    }

    /// Sets additional street information.
    pub fn street_additional_info(mut self, value: impl Into<String>) -> Self {
        self.street_additional_info = Some(value.into());
        self
    }

    /// Sets the house number.
    pub fn number(mut self, value: impl Into<String>) -> Self {
        self.number = Some(value.into());
        self
    }

    /// Sets the house number suffix.
    pub fn number_suffix(mut self, value: impl Into<String>) -> Self {
        self.number_suffix = Some(value.into());
        self
    }

    /// Sets the box number.
    pub fn box_number(mut self, value: BoxNumber) -> Self {
        self.box_number = Some(value);
        self
    }

    /// Sets the region.
    pub fn region(mut self, value: Region) -> Self {
        self.region = Some(value);
        self
    }

    /// Sets the company.
    pub fn company(mut self, value: impl Into<String>) -> Self {
        self.company = Some(value.into());
        self
    }

    /// Sets the phone number.
    pub fn phone(mut self, value: impl Into<String>) -> Self {
        self.phone = Some(value.into());
        self
    }

    /// Sets the email address.
    pub fn email(mut self, value: EmailAddress) -> Self {
        self.email = Some(value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn street_must_contain_a_non_digit() {
        assert!(Street::new("Antareslaan").is_ok());
        assert!(Street::new("1940").is_err());
        assert!(Street::new("A".repeat(41)).is_err());
    }

    #[test]
    fn email_shape() {
        assert!(EmailAddress::new("rob@example.net").is_ok());
        assert!(EmailAddress::new("rob@").is_err());
        assert!(EmailAddress::new("@example.net").is_err());
        assert!(EmailAddress::new("rob example@x.nl").is_err());
        assert!(EmailAddress::new("no-at-sign").is_err());
    }

    #[test]
    fn serializes_only_set_fields() {
        let recipient = Recipient::new(
            CountryCode::NL,
            PostalCode::new("2132 JE").unwrap(),
            City::new("Hoofddorp").unwrap(),
            Street::new("Antareslaan").unwrap(),
            Person::new("John Doe").unwrap(),
        )
        .number(HouseNumber::new("31").unwrap());
        assert_eq!(
            serde_json::to_string(&recipient).unwrap(),
            r#"{"cc":"NL","postal_code":"2132 JE","city":"Hoofddorp","street":"Antareslaan","person":"John Doe","number":"31"}"#
        );
    }
}
