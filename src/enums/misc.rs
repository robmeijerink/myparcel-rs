int_enum! {
    /// A position on an A4 label sheet.
    pub enum LabelQuadrant(u8) {
        TopLeftQuadrant = 1,
        TopRightQuadrant = 2,
        BottomLeftQuadrant = 3,
        BottomRightQuadrant = 4,
    }
}
int_enum! {
    /// What a customs declaration says the package contains.
    pub enum CustomsContents(u8) {
        CommercialGoods = 1,
        CommercialSample = 2,
        Documents = 3,
        Gift = 4,
        ReturnedGoods = 5,
    }
}
int_enum! {
    /// The type of a shipment.
    pub enum ShipmentType(u8) {
        Standard = 1,
        Return = 2,
        MultiCollo = 3,
        LabelPackage = 4,
        ReturnErs = 5,
        ReturnSpecial = 6,
        Express = 7,
        Fulfilment = 8,
    }
}
string_enum! {
    /// Whether a shipment is business-to-business or business-to-consumer.
    pub enum TransactionType {
        B2b = "B2B",
        B2c = "B2C",
    }
}
string_enum! {
    /// The direction of a shipment.
    pub enum Direction {
        Inbound = "INBOUND",
        Outbound = "OUTBOUND",
    }
}
string_enum! {
    /// The kind of location a shipment is picked up at.
    pub enum LocationType {
        Retail = "RETAIL",
        Locker = "LOCKER",
        PostOffice = "POST_OFFICE",
    }
}
