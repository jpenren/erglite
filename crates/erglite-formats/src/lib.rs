// SPDX-License-Identifier: MIT OR Apache-2.0
//! File formats: Zwift `.zwo` workouts and GPX routes (import), TCX sessions (export).
//!
//! FIT export is not implemented yet (work package WP6, see `docs/specification.md`).
#![forbid(unsafe_code)]

mod gpx;
mod tcx;
mod zwo;

pub use gpx::import_gpx_route;
pub use tcx::export_tcx;
pub use zwo::{ImportReport, import_zwo};

use thiserror::Error;

/// Errors raised while reading or writing files.
#[derive(Debug, Error)]
pub enum FormatError {
    /// The XML is malformed.
    #[error("invalid XML: {0}")]
    Xml(String),
    /// The content is valid XML but not usable.
    #[error("invalid content: {0}")]
    Invalid(String),
}

impl From<quick_xml::Error> for FormatError {
    fn from(e: quick_xml::Error) -> Self {
        FormatError::Xml(e.to_string())
    }
}
