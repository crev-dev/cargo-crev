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
    #[error("YAML: {}", _0)]
    YAML(#[from] serde_yaml::Error),

    #[error("YAML formatting: {}", _0)]
    YAMLFormat(Box<str>),

    #[error("Alternative source can't be empty")]
    AlternativeSourceCanTBeEmpty,
    #[error("Alternative name can't be empty")]
    AlternativeNameCanTBeEmpty,
    #[error("Issues with an empty `id` field are not allowed")]
    IssuesWithAnEmptyIDFieldAreNotAllowed,
    #[error("Advisories with no `id`s are not allowed")]
    AdvisoriesWithNoIDSAreNotAllowed,
    #[error("Advisories with an empty `id` field are not allowed")]
    AdvisoriesWithAnEmptyIDFieldAreNotAllowed,

    #[error("wrong length of crev id, expected 32 bytes, got {}", _0)]
    WrongIdLength(usize),

    #[error("Unknown level: {}", _0)]
    UnknownLevel(Box<str>),

    #[error("I/O: {}", _0)]
    IO(#[from] std::io::Error),

    #[error("Error building proof: {}", _0)]
    BuildingProof(Box<str>),
    #[error("Error building review: {}", _0)]
    BuildingReview(Box<str>),

    #[error("Invalid kind: {}, expected: {}", _0[0], _0[1])]
    InvalidKind(Box<[String; 2]>),
    #[error("Serialized to {} proofs", _0)]
    SerializedTooManyProofs(usize),

    #[error("Invalid CrevId: {}", _0)]
    InvalidCrevId(Box<str>),
    #[error("Invalid signature: {}", _0)]
    InvalidSignature(Box<str>),
    #[error("Invalid public key: {}", _0)]
    InvalidPublicKey(Box<str>),
    #[error("Invalid secret key: {}", _0)]
    InvalidSecretKey(Box<str>),
}
