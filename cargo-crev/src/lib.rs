// this is a dummy library just to make docs.rs render what I need
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![cfg_attr(
    feature = "documentation",
    doc = "See the [user documentation module](./doc/user/index.html) and in particular the [Getting Started Guide](./doc/user/getting_started/index.html)."
)]

#[cfg(feature = "documentation")]
/// Documentation
pub mod doc;
