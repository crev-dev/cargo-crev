/// # User documentation
///
/// New users are advise to start by reading the [Getting Started Guide](`self::user::getting_started`)
/// and [Glossary](`self::user::glossary`) modules.
///
/// Please be aware that all user documentation is
/// a continous work in progress, and can be incorrect
/// or stale.
///
/// Writting a high quality documentation is
/// a lot of work. Please help us! If you spot any
/// mistakes or ways to improve it:
///
/// 1. Open
/// [user documentation source code directory](https://github.com/dpc/crev/tree/master/cargo-crev/src/doc),
/// 2. Open the the affected file,
/// 3. Use *Edit this file* function,
/// 4. Modify the text,
/// 4. Click *Propose file change* button.
///
/// See the list of modules for the list of documented topis.
pub mod user {
    #[doc(include = "getting_started.md")]
    pub mod getting_started {}

    #[doc(include = "glossary.md")]
    pub mod glossary {}

    #[doc(include = "organizations.md")]
    pub mod organizations {}

    #[doc(include = "advisories.md")]
    pub mod advisories {}

    #[doc(include = "trust.md")]
    pub mod trust {}

    #[doc(include = "cargo_specific.md")]
    pub mod cargo_specific {}

    #[doc(include = "tips_and_tricks.md")]
    pub mod tips_and_tricks {}
}
