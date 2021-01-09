//! This module gather some helpers that helps interpret the roll result for certain RPG rules
//! This module can be empty if no helpers are activate by a feature flag
//! 
//!

#[cfg(feature = "ova")]
#[cfg_attr(docsrs, doc(cfg(feature = "ova")))]
/// Helpers for "OVA: The Anime Role-Playing Game result"
pub mod ova;
#[cfg(feature = "ova")]
pub use ova::*;

#[cfg(feature = "cde")]
#[cfg_attr(docsrs, doc(cfg(feature = "cde")))]
/// Helpers for "Chroniques de l'étrange"
pub mod cde;
#[cfg(feature = "cde")]
pub use cde::*;
