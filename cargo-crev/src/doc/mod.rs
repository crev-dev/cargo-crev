/// # User documentation
///
/// New users are advised to start by reading the [Getting Started Guide](`self::user::getting_started`)
/// and [Glossary](`self::user::glossary`) modules.
///
/// Please be aware that all user documentation is
/// a continous work in progress, and can be incorrect
/// or stale.
///
/// Writing a high quality documentation is
/// a lot of work. Please help us! If you spot any
/// mistakes or ways to improve it:
///
/// 1. Open
/// [user documentation source code directory](https://github.com/crev-dev/cargo-crev/tree/master/cargo-crev/src/doc),
/// 2. Open the affected file,
/// 3. Use *Edit this file* function,
/// 4. Modify the text,
/// 4. Click *Propose file change* button.
///
/// See the list of modules for the list of documented topics.
pub mod user {
    #[doc = include_str!("getting_started.md")]
    pub mod getting_started {}

    #[doc = include_str!("glossary.md")]
    pub mod glossary {}

    #[doc = include_str!("organizations.md")]
    pub mod organizations {}

    #[doc = include_str!("advisories.md")]
    pub mod advisories {}

    #[doc = include_str!("trust.md")]
    pub mod trust {}

    #[doc = include_str!("cargo_specific.md")]
    pub mod cargo_specific {}

    #[doc = include_str!("tips_and_tricks.md")]
    pub mod tips_and_tricks {}
}
