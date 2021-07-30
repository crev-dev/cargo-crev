use rand::{self, Rng};
use std::fmt;

pub fn random_id_str() -> String {
    let mut out = [0u8; 32];
    rand::thread_rng().fill(&mut out[..]);
    crev_common::base64_encode(&out)
}

pub fn write_comment_proof(comment: &str, f: &mut dyn fmt::Write) -> fmt::Result {
    if comment.is_empty() {
        return Ok(());
    }
    writeln!(f, "comment: |-")?;
    for line in comment.lines() {
        writeln!(f, "  {}", line)?;
    }
    Ok(())
}

pub fn write_comment_draft(comment: &str, f: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(f, "comment: |-")?;
    for line in comment.lines() {
        writeln!(f, "  {}", line)?;
    }
    if comment.is_empty() {
        writeln!(f, "  ")?;
    }
    Ok(())
}

#[macro_export]
macro_rules! serde_content_serialize {
    ($self: ident, $fmt: ident) => {
        // Remove comment for manual formatting
        let mut clone = $self.clone();
        let mut comment = String::new();
        std::mem::swap(&mut comment, &mut clone.comment);

        if clone.common.kind.is_none() {
            clone.common.kind = Some(Self::KIND.into());
        }

        crev_common::serde::write_as_headerless_yaml(&clone, $fmt)?;
        $crate::util::write_comment_proof(comment.as_str(), $fmt)?;
    };
}

#[macro_export]
macro_rules! serde_draft_serialize {
    ($self: ident, $fmt: ident) => {
        // Remove comment for manual formatting
        let mut clone = $self.clone();
        let mut comment = String::new();
        std::mem::swap(&mut comment, &mut clone.comment);

        crev_common::serde::write_as_headerless_yaml(&clone, $fmt)?;
        $crate::util::write_comment_draft(comment.as_str(), $fmt)?;
    };
}
