use id;

#[test]
fn lock_and_unlock() -> Result<()> {
    let id = id::OwnId::generate("Dawid Ciężarkiewicz".into());

    let id_relocked = id.to_locked("password")?.to_unlocked("password")?;
    assert_eq!(id.pub_key_as_bytes(), id_relocked.pub_key_as_bytes());

    assert!(
        id.to_locked("password")?
            .to_unlocked("wrongpassword")
            .is_err()
    );

    let id_stored = serde_yaml::to_string(&id.to_locked("pass")?)?;
    let id_restored: id::OwnId = serde_yaml::from_str::<LockedId>(&id_stored)?.to_unlocked("pass")?;

    println!("{}", id_stored);

    assert_eq!(id.pub_key_as_bytes(), id_restored.pub_key_as_bytes());
    Ok(())
}
