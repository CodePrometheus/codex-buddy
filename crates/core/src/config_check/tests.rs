use super::*;
use std::path::PathBuf;
use tempfile::tempdir;

fn write_cfg(dir: &Path, body: &str) -> PathBuf {
    let p = dir.join("config.toml");
    fs::write(&p, body).unwrap();
    p
}

#[test]
fn missing_file_defaults_to_file() {
    let d = tempdir().unwrap();
    assert_eq!(
        credential_store(&d.path().join("nope.toml")).unwrap(),
        CredentialStore::File
    );
}

#[test]
fn unset_defaults_to_file() {
    let d = tempdir().unwrap();
    let p = write_cfg(d.path(), "model = \"gpt-5\"\n");
    assert_eq!(credential_store(&p).unwrap(), CredentialStore::File);
    assert!(ensure_file_store(&p).is_ok());
}

#[test]
fn parses_variants() {
    let d = tempdir().unwrap();

    let p = write_cfg(d.path(), "cli_auth_credentials_store = \"keyring\"\n");
    assert_eq!(credential_store(&p).unwrap(), CredentialStore::Keyring);
    assert!(ensure_file_store(&p).is_err());

    let p = write_cfg(d.path(), "cli_auth_credentials_store='auto' # comment\n");
    assert_eq!(credential_store(&p).unwrap(), CredentialStore::Auto);

    let p = write_cfg(d.path(), "  cli_auth_credentials_store   =   \"file\"  \n");
    assert!(ensure_file_store(&p).is_ok());
}

#[test]
fn ignores_commented_and_prefixed_keys() {
    let d = tempdir().unwrap();
    let p = write_cfg(
        d.path(),
        "# cli_auth_credentials_store = \"keyring\"\ncli_auth_credentials_store_extra = \"x\"\n",
    );
    assert_eq!(credential_store(&p).unwrap(), CredentialStore::File);
}
