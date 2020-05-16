use crate::{
    proof,
    proof::content::{ValidationError, ValidationResult},
    serde_content_serialize, serde_draft_serialize, Error, Level, ParseError,
};
use crev_common::{self, is_equal_default, is_set_empty, is_vec_empty};
use derive_builder::Builder;
use proof::{CommonOps, Content};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, default::Default, fmt, ops};
use typed_builder::TypedBuilder;

const CURRENT_PACKAGE_REVIEW_PROOF_SERIALIZATION_VERSION: i64 = -1;

fn cur_version() -> i64 {
    CURRENT_PACKAGE_REVIEW_PROOF_SERIALIZATION_VERSION
}

/// Possible flags to mark on the package
#[derive(Clone, Builder, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Flags {
    #[serde(default = "Default::default", skip_serializing_if = "is_equal_default")]
    pub unmaintained: bool,
}

impl ops::Add<Flags> for Flags {
    type Output = Self;
    fn add(self, other: Flags) -> Self {
        Self {
            unmaintained: self.unmaintained || other.unmaintained,
        }
    }
}

impl From<FlagsDraft> for Flags {
    fn from(flags: FlagsDraft) -> Self {
        Self {
            unmaintained: flags.unmaintained,
        }
    }
}
/// Like `Flags` but serializes all fields every time
#[derive(Clone, Builder, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct FlagsDraft {
    #[serde(default = "Default::default")]
    unmaintained: bool,
}

impl From<Flags> for FlagsDraft {
    fn from(flags: Flags) -> Self {
        Self {
            unmaintained: flags.unmaintained,
        }
    }
}

/// Body of a Package Review Proof
#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
pub struct Package {
    #[serde(flatten)]
    pub common: proof::Common,
    #[serde(rename = "package")]
    pub package: proof::PackageInfo,
    #[serde(skip_serializing_if = "Option::is_none", default = "Default::default")]
    #[serde(rename = "package-diff-base")]
    #[builder(default = "Default::default()")]
    pub diff_base: Option<proof::PackageInfo>,
    #[builder(default = "Default::default()")]
    #[serde(default = "Default::default", skip_serializing_if = "is_equal_default")]
    pub review: super::Review,
    #[builder(default = "Default::default()")]
    #[serde(skip_serializing_if = "is_vec_empty", default = "Default::default")]
    pub issues: Vec<Issue>,
    #[builder(default = "Default::default()")]
    #[serde(skip_serializing_if = "is_vec_empty", default = "Default::default")]
    pub advisories: Vec<Advisory>,
    #[serde(default = "Default::default", skip_serializing_if = "is_equal_default")]
    #[builder(default = "Default::default()")]
    pub flags: Flags,
    #[builder(default = "Default::default()")]
    #[serde(skip_serializing_if = "is_set_empty", default = "Default::default")]
    pub alternatives: HashSet<proof::PackageId>,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    pub comment: String,
}

impl PackageBuilder {
    pub fn from<VALUE: Into<crate::PublicId>>(&mut self, value: VALUE) -> &mut Self {
        if let Some(ref mut common) = self.common {
            common.from = value.into();
        } else {
            self.common = Some(proof::Common {
                kind: Some(Package::KIND.into()),
                version: cur_version(),
                date: crev_common::now(),
                from: value.into(),
            });
        }
        self
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.serialize_to(f).map_err(|_| fmt::Error)
    }
}

impl proof::WithReview for Package {
    fn review(&self) -> &super::Review {
        &self.review
    }
}

impl proof::CommonOps for Package {
    fn common(&self) -> &proof::Common {
        &self.common
    }

    fn kind(&self) -> &str {
        // Backfill the `kind` if it is empty (legacy format)
        self.common.kind.as_deref().unwrap_or(Self::KIND)
    }
}

/// Like `Package` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Draft {
    #[serde(default = "Default::default", skip_serializing_if = "is_equal_default")]
    review: super::Review,
    #[serde(default = "Default::default", skip_serializing_if = "is_vec_empty")]
    pub advisories: Vec<Advisory>,
    #[serde(default = "Default::default", skip_serializing_if = "is_vec_empty")]
    pub issues: Vec<Issue>,
    #[serde(default = "Default::default", skip_serializing_if = "String::is_empty")]
    comment: String,
    #[serde(default = "Default::default")]
    pub flags: FlagsDraft,
    #[serde(default = "Default::default", skip_serializing_if = "is_set_empty")]
    pub alternatives: HashSet<proof::PackageId>,
}

impl Draft {
    pub fn parse(s: &str) -> std::result::Result<Self, ParseError> {
        serde_yaml::from_str(&s).map_err(ParseError::Draft)
    }
}

impl From<Package> for Draft {
    fn from(package: Package) -> Self {
        Draft {
            review: package.review,
            advisories: package.advisories,
            issues: package.issues,
            comment: package.comment,
            alternatives: if package.alternatives.is_empty() {
                // To give user a convenient template, we pre-fill with the same `source`,
                // and an empty `name`. If undedited, this entry will be deleted on parsing.
                vec![proof::PackageId {
                    source: package.package.id.id.source,
                    name: "".into(),
                }]
                .into_iter()
                .collect()
            } else {
                package.alternatives
            },
            flags: package.flags.into(),
        }
    }
}

impl proof::Content for Package {
    fn validate_data(&self) -> ValidationResult<()> {
        self.ensure_kind_is(Self::KIND)?;

        for alternative in &self.alternatives {
            if alternative.source.is_empty() {
                Err(ValidationError::AlternativeSourceCanNotBeEmpty)?;
            }
            if alternative.name.is_empty() {
                Err(ValidationError::AlternativeNameCanNotBeEmpty)?;
            }
        }
        for issue in &self.issues {
            if issue.id.is_empty() {
                Err(ValidationError::IssuesWithAnEmptyIDFieldAreNotAllowed)?;
            }
        }

        for advisory in &self.advisories {
            if advisory.ids.is_empty() {
                Err(ValidationError::AdvisoriesWithNoIDSAreNotAllowed)?;
            }

            for id in &advisory.ids {
                if id.is_empty() {
                    Err(ValidationError::AdvisoriesWithAnEmptyIDFieldAreNotAllowed)?;
                }
            }
        }
        Ok(())
    }

    fn serialize_to(&self, fmt: &mut dyn std::fmt::Write) -> fmt::Result {
        serde_content_serialize!(self, fmt);
        Ok(())
    }
}

impl proof::ContentWithDraft for Package {
    fn to_draft(&self) -> proof::Draft {
        proof::Draft {
            title: format!(
                "Package Review of {} {}",
                self.package.id.id.name, self.package.id.version
            ),
            body: Draft::from(self.clone()).to_string(),
        }
    }

    fn apply_draft(&self, s: &str) -> Result<Self, Error> {
        let draft = Draft::parse(&s)?;

        let mut package = self.clone();
        package.review = draft.review;
        package.comment = draft.comment;
        package.advisories = draft.advisories;
        package.issues = draft.issues;
        package.alternatives = draft
            .alternatives
            .into_iter()
            .filter(|a| !a.name.is_empty())
            .collect();
        package.flags = draft.flags.into();

        package.validate_data()?;
        Ok(package)
    }
}

impl Package {
    pub const KIND: &'static str = "package review";

    pub fn is_advisory_for(&self, version: &Version) -> bool {
        for advisory in &self.advisories {
            if advisory.is_for_version_when_reported_in_version(version, &self.package.id.version) {
                return true;
            }
        }
        false
    }
}

impl fmt::Display for Draft {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        serde_draft_serialize!(self, fmt);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum VersionRange {
    Minor,
    Major,
    All,
}

#[derive(Debug, Clone)]
pub struct VersionRangeParseError(());

impl fmt::Display for VersionRangeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Could not parse an incorrect advisory range value")
    }
}

impl Default for VersionRange {
    fn default() -> Self {
        VersionRange::All
    }
}

impl std::str::FromStr for VersionRange {
    type Err = VersionRangeParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "all" => VersionRange::All,
            "major" => VersionRange::Major,
            "minor" => VersionRange::Minor,
            _ => return Err(VersionRangeParseError(())),
        })
    }
}

impl VersionRange {
    fn all() -> Self {
        VersionRange::All
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_all_ref(&self) -> bool {
        VersionRange::All == *self
    }
}

/// Advisory to upgrade to the package version
///
/// Advisory means a general important fix was included in this
/// release, and all previous releases were potentially affected.
/// We don't play with exact ranges.
#[derive(Clone, TypedBuilder, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Advisory {
    pub ids: Vec<String>,
    #[builder(default)]
    pub severity: Level,

    #[builder(default)]
    #[serde(
        default = "VersionRange::all",
        skip_serializing_if = "VersionRange::is_all_ref"
    )]
    pub range: VersionRange,

    #[builder(default)]
    #[serde(default = "Default::default")]
    pub comment: String,
}

impl From<VersionRange> for Advisory {
    fn from(r: VersionRange) -> Self {
        Advisory {
            range: r,
            ..Default::default()
        }
    }
}

impl Default for Advisory {
    fn default() -> Self {
        Self {
            ids: vec![],
            range: VersionRange::default(),
            severity: Default::default(),
            comment: "".to_string(),
        }
    }
}

impl Advisory {
    pub fn is_for_version_when_reported_in_version(
        &self,
        for_version: &Version,
        in_pkg_version: &Version,
    ) -> bool {
        if for_version < in_pkg_version {
            match self.range {
                VersionRange::All => return true,
                VersionRange::Major => {
                    if in_pkg_version.major == for_version.major {
                        return true;
                    }
                }
                VersionRange::Minor => {
                    if in_pkg_version.major == for_version.major
                        && in_pkg_version.minor == for_version.minor
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Issue with a package version
///
/// `Issue` is a kind of opposite of [`Advisory`]. It reports
/// a problem with package in a given version. It leaves the
/// question open if any previous and following versions might
/// also be affected, but will be considered open and affecting
/// all following versions withing the `range` until an advisory
/// is found for it, matching the id.
#[derive(Clone, TypedBuilder, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Issue {
    pub id: String,
    #[builder(default)]
    pub severity: Level,

    #[builder(default)]
    #[serde(
        default = "VersionRange::all",
        skip_serializing_if = "VersionRange::is_all_ref"
    )]
    pub range: VersionRange,

    #[builder(default)]
    #[serde(default = "Default::default")]
    pub comment: String,
}

impl Issue {
    pub fn new(id: String) -> Self {
        Self {
            id,
            range: Default::default(),
            severity: Default::default(),
            comment: Default::default(),
        }
    }
    pub fn new_with_severity(id: String, severity: Level) -> Self {
        Self {
            id,
            range: Default::default(),
            severity,
            comment: Default::default(),
        }
    }
    pub fn is_for_version_when_reported_in_version(
        &self,
        for_version: &Version,
        in_pkg_version: &Version,
    ) -> bool {
        if for_version >= in_pkg_version {
            match self.range {
                VersionRange::All => return true,
                VersionRange::Major => {
                    if in_pkg_version.major == for_version.major {
                        return true;
                    }
                }
                VersionRange::Minor => {
                    if in_pkg_version.major == for_version.major
                        && in_pkg_version.minor == for_version.minor
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
}
