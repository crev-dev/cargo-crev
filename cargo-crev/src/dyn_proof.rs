use std::fmt;

use anyhow::{Result, bail};
use crev_data::{
    PublicId, UnlockedId,
    proof::{self, CommonOps, ContentExt},
};

pub fn parse_dyn_content(proof: &proof::Proof) -> Result<Box<dyn DynContent>> {
    Ok(match proof.kind() {
        proof::CodeReview::KIND => Box::new(proof.parse_content::<proof::review::Code>()?),
        proof::PackageReview::KIND => Box::new(proof.parse_content::<proof::review::Package>()?),
        proof::Trust::KIND => Box::new(proof.parse_content::<proof::Trust>()?),
        kind => bail!("Unsupported proof kind: {}", kind),
    })
}

/// Type-erased proof body that can be mutated, signed, displayed, and
/// validated through `crev_data::proof::ContentExt`.
///
/// Inheriting from [`proof::Content`] and [`fmt::Display`] lets a
/// `&dyn DynContent` flow directly into [`crate::shared::maybe_store`]
/// (which is generic over `C: Content + Display + ?Sized`), so the
/// dispatch logic for storing/printing lives where it belongs — at the
/// call site — instead of being a method on this trait.
pub trait DynContent: proof::Content + fmt::Display {
    fn set_date(&mut self, date: &proof::Date);
    fn set_author(&mut self, id: &PublicId);
    fn sign_by(&self, id: &UnlockedId) -> Result<proof::Proof>;
}

impl DynContent for proof::review::Code {
    fn set_date(&mut self, date: &proof::Date) {
        self.common.date = *date;
    }
    fn set_author(&mut self, id: &PublicId) {
        self.common.from = id.clone();
    }
    fn sign_by(&self, id: &UnlockedId) -> Result<proof::Proof> {
        Ok(ContentExt::sign_by(self, id)?)
    }
}
impl DynContent for proof::review::Package {
    fn set_date(&mut self, date: &proof::Date) {
        self.common.date = *date;
    }
    fn set_author(&mut self, id: &PublicId) {
        self.common.from = id.clone();
    }
    fn sign_by(&self, id: &UnlockedId) -> Result<proof::Proof> {
        Ok(ContentExt::sign_by(self, id)?)
    }
}
impl DynContent for proof::trust::Trust {
    fn set_date(&mut self, date: &proof::Date) {
        self.common.date = *date;
    }
    fn set_author(&mut self, id: &PublicId) {
        self.common.from = id.clone();
    }
    fn sign_by(&self, id: &UnlockedId) -> Result<proof::Proof> {
        Ok(ContentExt::sign_by(self, id)?)
    }
}
