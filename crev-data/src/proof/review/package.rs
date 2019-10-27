use crate::{proof, Level, Result};
use crev_common::{self, is_equal_default, is_vec_empty};
use derive_builder::Builder;
use failure::bail;
use proof::{CommonOps, Content};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::{default::Default, fmt, mem};
use typed_builder::TypedBuilder;

const CURRENT_PACKAGE_REVIEW_PROOF_SERIALIZATION_VERSION: i64 = -1;

fn cur_version() -> i64 {
    CURRENT_PACKAGE_REVIEW_PROOF_SERIALIZATION_VERSION
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
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    pub comment: String,
}

impl PackageBuilder {
    pub fn from<VALUE: Into<crate::PubId>>(&mut self, value: VALUE) -> &mut Self {
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
        self.common
            .kind
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(Self::KIND)
    }
}

/// Like `Package` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Draft {
    review: super::Review,
    #[serde(default = "Default::default", skip_serializing_if = "is_vec_empty")]
    pub advisories: Vec<Advisory>,
    #[serde(default = "Default::default", skip_serializing_if = "is_vec_empty")]
    pub issues: Vec<Issue>,
    #[serde(default = "Default::default", skip_serializing_if = "String::is_empty")]
    comment: String,
}

impl Draft {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}

impl From<Package> for Draft {
    fn from(package: Package) -> Self {
        Draft {
            review: package.review,
            advisories: package.advisories,
            issues: package.issues,
            comment: package.comment,
        }
    }
}

impl proof::Content for Package {
    fn validate_data(&self) -> Result<()> {
        self.ensure_kind_is(Self::KIND)?;
        for issue in &self.issues {
            if issue.id.is_empty() {
                bail!("Issues with an empty `id` field are not allowed");
            }
        }

        for advisory in &self.advisories {
            if advisory.ids.is_empty() {
                bail!("Advisories with no `id`s are not allowed");
            }

            for id in &advisory.ids {
                if id.is_empty() {
                    bail!("Advisories with an empty `id` field are not allowed");
                }
            }
        }
        Ok(())
    }

    fn serialize_to(&self, fmt: &mut dyn std::fmt::Write) -> Result<()> {
        if self.common.kind.is_none() {
            // backfill during serialization
            let mut copy = self.clone();
            copy.common.kind = Some(Self::KIND.into());
            Ok(crev_common::serde::write_as_headerless_yaml(&self, fmt)?)
        } else {
            Ok(crev_common::serde::write_as_headerless_yaml(&self, fmt)?)
        }
    }
}

impl proof::ContentWithDraft for Package {
    fn to_draft(&self) -> proof::Draft {
        proof::Draft {
            title: format!(
                "Package Review of {} {}",
                self.package.name, self.package.version
            ),
            body: Draft::from(self.clone()).to_string(),
        }
    }

    fn apply_draft(&self, s: &str) -> Result<Self> {
        let draft = Draft::parse(&s)?;

        let mut copy = self.clone();
        copy.review = draft.review;
        copy.comment = draft.comment;
        copy.advisories = draft.advisories;
        copy.issues = draft.issues;

        copy.validate_data()?;
        Ok(copy)
    }
}

impl Package {
    pub const KIND: &'static str = "package review";

    pub fn is_advisory_for(&self, version: &Version) -> bool {
        for advisory in &self.advisories {
            if advisory.is_for_version_when_reported_in_version(version, &self.package.version) {
                return true;
            }
        }
        false
    }
}

fn write_comment(comment: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "comment: |")?;
    for line in comment.lines() {
        writeln!(f, "  {}", line)?;
    }
    if comment.is_empty() {
        writeln!(f, "  ")?;
    }
    Ok(())
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Remove comment for manual formatting
        let mut clone = self.clone();
        let mut comment = String::new();
        mem::swap(&mut comment, &mut clone.comment);

        crev_common::serde::write_as_headerless_yaml(&clone, f)?;
        write_comment(comment.as_str(), f)
    }
}

impl fmt::Display for Draft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Remove comment for manual formatting
        let mut clone = self.clone();
        let mut comment = String::new();
        mem::swap(&mut comment, &mut clone.comment);

        crev_common::serde::write_as_headerless_yaml(&clone, f)?;
        write_comment(comment.as_str(), f)
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
