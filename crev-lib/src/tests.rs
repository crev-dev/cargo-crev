use super::*;

use crev_data::id::{OwnId };

#[test]
fn lock_and_unlock() -> Result<()> {
    let id = OwnId::generate("Dawid Ciężarkiewicz".into());

    let id_relocked = id::LockedId::from_own_id(&id, "password")?.to_unlocked("password")?;
    assert_eq!(id.id.id, id_relocked.id.id);

    assert!(
        id::LockedId::from_own_id(&id, "password")?
            .to_unlocked("wrongpassword")
            .is_err()
    );

    let id_stored = serde_yaml::to_string(&id::LockedId::from_own_id(&id, "pass")?)?;
    let id_restored: OwnId = serde_yaml::from_str::<id::LockedId>(&id_stored)?.to_unlocked("pass")?;

    println!("{}", id_stored);

    assert_eq!(id.id.id, id_restored.id.id);
    Ok(())
}
