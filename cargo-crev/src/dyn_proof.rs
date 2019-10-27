use common_failures::Result;
use crev_data::proof::CommonOps;
use crev_data::{
    proof::{self, ContentExt},
    OwnId, PubId,
};
use failure::bail;

pub fn parse_dyn_content(proof: &proof::Proof) -> Result<Box<dyn DynContent>> {
    Ok(match proof.kind() {
        proof::CodeReview::KIND => Box::new(proof.parse_content::<proof::review::Code>()?),
        proof::PackageReview::KIND => Box::new(proof.parse_content::<proof::review::Package>()?),
        proof::Trust::KIND => Box::new(proof.parse_content::<proof::Trust>()?),
        kind => bail!("Unsupported proof kind: {}", kind),
    })
}

pub trait DynContent {
    fn set_date(&mut self, date: &proof::Date);
    fn set_author(&mut self, id: &PubId);
    fn sign_by(&self, id: &OwnId) -> Result<proof::Proof>;
}

impl DynContent for proof::review::Code {
    fn set_date(&mut self, date: &proof::Date) {
        self.common.date = date.clone();
    }
    fn set_author(&mut self, id: &PubId) {
        self.common.from = id.clone();
    }
    fn sign_by(&self, id: &OwnId) -> Result<proof::Proof> {
        Ok(ContentExt::sign_by(self, id)?)
    }
}
impl DynContent for proof::review::Package {
    fn set_date(&mut self, date: &proof::Date) {
        self.common.date = date.clone();
    }
    fn set_author(&mut self, id: &PubId) {
        self.common.from = id.clone();
    }
    fn sign_by(&self, id: &OwnId) -> Result<proof::Proof> {
        Ok(ContentExt::sign_by(self, id)?)
    }
}
impl DynContent for proof::trust::Trust {
    fn set_date(&mut self, date: &proof::Date) {
        self.common.date = date.clone();
    }
    fn set_author(&mut self, id: &PubId) {
        self.common.from = id.clone();
    }
    fn sign_by(&self, id: &OwnId) -> Result<proof::Proof> {
        Ok(ContentExt::sign_by(self, id)?)
    }
}
