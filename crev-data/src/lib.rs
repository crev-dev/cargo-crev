//! This crate contains only code handling data types
//! used by `crev`, without getting into details
//! how actually `crev` works (where and how it manages data).

pub mod digest;
pub mod id;
pub mod level;
mod prelude;
pub mod proof;
pub mod url;
#[macro_use]
pub mod util;
use crate::{id::IdError, proof::content::ValidationError};
pub use semver::Version;

pub use crate::{
    digest::Digest,
    id::{Id, PublicId, UnlockedId},
    level::Level,
    proof::{
        review,
        review::{Rating, Review},
        trust::TrustLevel,
    },
    url::Url,
};

#[cfg(test)]
mod tests;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("`kind` field missing")]
    KindFieldMissing,

    #[error("Unexpected `kind` value in a legacy format")]
    UnexpectedKindValueInALegacyFormat,

    #[error("Parsing error when looking for start of code review proof")]
    ParsingErrorWhenLookingForStartOfCodeReviewProof,

    #[error("Parsing error: type name mismatch in the signature")]
    ParsingErrorTypeNameMismatchInTheSignature,
    #[error("Parsing error: type name mismatch in the footer")]
    ParsingErrorTypeNameMismatchInTheFooter,
    #[error("Signature too long")]
    SignatureTooLong,
    #[error("Unexpected EOF while parsing")]
    UnexpectedEOFWhileParsing,
    #[error("Proof body too long")]
    ProofBodyTooLong,

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error("YAML formatting: {}", _0)]
    YAMLFormat(Box<str>),

    #[error(transparent)]
    Id(#[from] IdError),

    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error("Unknown level: {}", _0)]
    UnknownLevel(Box<str>),

    #[error("I/O: {}", _0)]
    IO(#[from] std::io::Error),

    #[error("Error building proof: {}", _0)]
    BuildingProof(Box<str>),

    #[error("Error building review: {}", _0)]
    BuildingReview(Box<str>),

    #[error("Serialized to {} proofs", _0)]
    SerializedTooManyProofs(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Draft parse error: {}", _0)]
    Draft(#[source] serde_yaml::Error),

    #[error("Proof parse error: {}", _0)]
    Proof(#[source] serde_yaml::Error),
}
