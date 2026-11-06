int_enum! {
    /// The status of a shipment.
    ///
    /// Codes `39` to `42` are listed by the API without a name and are reported as unknown.
    pub enum ShipmentStatus(u8) {
        /// Pending - Concept
        PendingConcept = 1,
        /// Pending - Registered
        PendingRegistered = 2,
        /// En Route - Handed to Carrier
        EnrouteHandedToCarrier = 3,
        /// En Route - Sorting
        EnrouteSorting = 4,
        /// En Route - Distribution
        EnrouteDistribution = 5,
        /// En Route - Customs
        DeliveredAtCustoms = 6,
        /// Delivered - At Recipient
        Delivered = 7,
        /// Delivered - Ready for Pickup
        InPickupLocation = 8,
        /// Delivered - Picked Up
        PickedUp = 9,
        /// Delivered - Return Shipment Ready for Pickup
        ReturnShipmentDelivered = 10,
        /// Delivered - Return Shipment Picked Up
        ReturnShipmentInPickupLocation = 11,
        /// Printed - Unpaid Label
        PrintedUnpaidLabel = 12,
        /// Credited
        Credit = 13,
        /// Printed - Digital Stamp
        PrintedStamp = 14,
        /// Printed - External Shipment
        PrintedExternalShipment = 15,
        /// Expired
        Expired = 16,
        /// Cancelled
        Cancelled = 17,
        /// Printed - Untracked Shipment
        PrintedUntrackedShipment = 18,
        /// Delivered - At Agreed Location
        DeliveredAtAgreedLocation = 19,
        /// (legacy) Pending - Concept
        LegacyPendingConcept = 30,
        /// (legacy) Pending - Registered
        LegacyPendingRegistered = 31,
        /// (legacy) En Route - Handed to Carrier
        LegacyEnrouteHandedToCarrier = 32,
        /// (legacy) En Route - Sorting
        LegacyEnrouteSorting = 33,
        /// (legacy) En Route - Distribution
        LegacyEnrouteDistribution = 34,
        /// (legacy) En Route - Customs
        LegacyDeliveredAtCustoms = 35,
        /// (legacy) Delivered - At Recipient
        LegacyDelivered = 36,
        /// (legacy) Delivered - Ready for Pickup
        LegacyInPickupLocation = 37,
        /// (legacy) Delivered - Picked Up
        LegacyPickedUp = 38,
        /// Inactive
        Inactive = 98,
        /// Deleted
        Deleted = 99,
        /// Credit - Rejected
        CreditRejected = 100,
        /// Credit - Approved
        CreditApproved = 101,
    }
}
string_enum! {
    /// The simplified status of a shipment.
    pub enum MainStatus {
        Registered = "registered",
        HandedToCarrier = "handed_to_carrier",
        Sorting = "sorting",
        Distribution = "distribution",
        Delivered = "delivered",
    }
}
string_enum! {
    /// Whether a shipment has been paid for.
    pub enum TransactionStatus {
        Pending = "pending",
        Unpaid = "unpaid",
        Paid = "paid",
    }
}
