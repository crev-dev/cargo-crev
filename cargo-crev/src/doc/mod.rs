/// # User documentation
///
/// For the new users it's recommended to read the [Getting Started Guide](`self::user::getting_started`).
///
/// See the list of modules for the list of documented topis.
pub mod user {
    #[doc(include = "doc/getting_started.md")]
    pub mod getting_started {}
    #[doc(include = "doc/organizations.md")]
    pub mod organizations {}
    #[doc(include = "doc/advisories.md")]
    pub mod advisories {}
    #[doc(include = "doc/trust.md")]
    pub mod trust {}
    #[doc(include = "doc/cargo_specific.md")]
    pub mod cargo_specific {}
}
