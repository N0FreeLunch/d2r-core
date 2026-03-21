//! Gateway for external item data (Deep Links, Clipboard).
//! This module provides the "Sandbox" zone for validating incoming item payloads.

use crate::item::{Item, HuffmanTree};
use crate::error::DiagnosticError;
use std::io;

/// Payload represents the raw input from a Deep Link.
/// d2r-core://import/item?data=<hex_payload>
pub struct ItemGateway;

impl ItemGateway {
    /// Safe entry point for importing an item from a hex-encoded bitstream.
    /// This uses the VO-guarded parsing logic and returns a DiagnosticError
    /// if the payload is malicious or malformed.
    pub fn from_payload(hex_data: &str) -> Result<Item, io::Error> {
        let bytes = hex::decode(hex_data).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid HEX payload: {}", e))
        })?;

        let huffman = HuffmanTree::new();
        
        // Use the reinforced Item::read_single_item (Total Function approach)
        // to prevent panics and return structured errors.
        Item::from_bytes(&bytes, &huffman)
    }

    /// Verifies if the imported item can be placed in the current inventory.
    /// This is an E2E "Placement Guard" check.
    pub fn verify_placement(
        _item: &Item,
        _x: u8,
        _y: u8,
    ) -> Result<crate::domain::vo::InventoryPlacement, &'static str> {
        // This will eventually fetch item dimensions from dataシグ(Template)
        // and reconcile it with the requested VO coordinate.
        // Placeholder for the next architectural slice.
        Err("Placement verification not yet fully linked to templates")
    }
}
