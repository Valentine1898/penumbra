#![deny(clippy::unwrap_used)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg_attr(docsrs, doc(cfg(feature = "component")))]
#[cfg(feature = "component")]
pub mod component;

pub mod event;
pub mod state_key;

mod fee;

pub use fee::Fee;
