use common_failures::Result;
use crev_data::{
    proof::{self, ContentExt},
    OwnId, PubId,
};

pub fn parse_dyn_content(proof: &proof::Proof) -> Result<Box<dyn DynContent>> {
    Ok(match proof.type_name() {
        "code review" => Box::new(proof.parse_content::<proof::review::Code>()?),
        _ => unimplemented!(),
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
