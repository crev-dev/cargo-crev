//! Bunch of code that is auxiliary and common for all `crev`

pub mod serde;
extern crate base64;
extern crate hex;
extern crate chrono;


/// Now with a fixed offset of the current system timezone
pub fn now() -> chrono::DateTime<chrono::offset::FixedOffset> {
    let date = chrono::offset::Local::now();
    date.with_timezone(&date.offset())
}

