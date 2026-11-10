use aqvn_core::secret::{SecretKey, SecretString, TokenCredentials};
use aqvn_core::vault::{
    test_keyring_vault, EnvelopeCipher, KdfParams, KeyringBackendKind, KeyringVault, VaultError,
};
use tempfile::tempdir;

#[test]
fn test_keyring_vault_contract_passes() {
    let res = test_keyring_vault();
    assert!(res.is_ok(), "keyring vault contract failed: {res:?}");
}

#[test]
fn test_in_memory_vault_crud() {
    let vault = KeyringVault::in_memory("aqvn-unit-test");
    assert_eq!(vault.backend_kind(), KeyringBackendKind::InMemory);

    let account = "api_key_primary";
    let secret = SecretString::new("aqvn-secret-value-xyz-9876");

    // Store
    vault.set_secret(account, &secret).expect("store secret");

    // Retrieve
    let loaded = vault.get_secret(account).expect("retrieve secret");
    assert!(loaded.is_some());
    assert!(secret.constant_time_eq(&loaded.unwrap()));

    // Delete
    vault.delete_secret(account).expect("delete secret");
    let after_delete = vault.get_secret(account).expect("check deleted");
    assert!(after_delete.is_none());
}

#[test]
fn test_encrypted_file_vault_persistence_and_wrong_password() {
    let dir = tempdir().expect("create temp dir");
    let vault_file = dir.path().join("test_vault.enc");
    let passphrase = SecretString::new("strong-master-passphrase-2026");

    let account = "ssh_tunnel_key";
    let secret = SecretString::new("ssh-ed25519-secret-material-12345");

    // Phase 1: Create vault, store secret
    {
        let vault =
            KeyringVault::open_encrypted_file("aqvn-file-test", vault_file.clone(), &passphrase)
                .expect("open file vault");
        assert_eq!(
            vault.backend_kind(),
            KeyringBackendKind::EncryptedEnvelopeFile
        );
        vault
            .set_secret(account, &secret)
            .expect("store in file vault");
    }

    // Phase 2: Reopen vault with same passphrase, verify secret
    {
        let vault =
            KeyringVault::open_encrypted_file("aqvn-file-test", vault_file.clone(), &passphrase)
                .expect("reopen file vault");
        let loaded = vault
            .get_secret(account)
            .expect("get secret from file vault");
        assert!(loaded.is_some());
        assert!(secret.constant_time_eq(&loaded.unwrap()));
    }

    // Phase 3: Reopen with wrong passphrase, expect AuthenticationFailed
    {
        let wrong_pass = SecretString::new("incorrect-passphrase");
        let wrong_vault =
            KeyringVault::open_encrypted_file("aqvn-file-test", vault_file, &wrong_pass)
                .expect("open with wrong passphrase");
        let result = wrong_vault.get_secret(account);
        assert!(
            matches!(result, Err(VaultError::AuthenticationFailed)),
            "expected AuthenticationFailed, got: {result:?}"
        );
    }
}

#[test]
fn test_token_credentials_bundle_serialization() {
    let creds = TokenCredentials::new(
        "aqvn-service",
        "worker-node-1",
        SecretString::new("tok-abc-123"),
    )
    .with_rfc_password(SecretString::new("rfc-pass-999"))
    .with_ssh_password(SecretString::new("ssh-pass-888"));

    let vault = KeyringVault::in_memory("aqvn-creds-test");
    vault.store_credentials(&creds).expect("store credentials");

    let retrieved = vault
        .load_credentials("worker-node-1")
        .expect("load credentials")
        .expect("credentials should be found");

    assert_eq!(retrieved.service, "aqvn-service");
    assert_eq!(retrieved.account, "worker-node-1");
    assert!(creds.token.constant_time_eq(&retrieved.token));
    assert!(retrieved.rfc_password.is_some());
    assert!(retrieved.ssh_password.is_some());
}

#[test]
fn test_chacha20_poly1305_tamper_detection() {
    let key = SecretKey::generate_random().expect("generate random key");
    let aad = b"test-aad-metadata";
    let plaintext = b"sensitive payload data";

    let mut sealed = EnvelopeCipher::seal_with_key(&key, aad, plaintext).expect("seal with key");

    // Tamper with the last byte (Poly1305 tag)
    let last_idx = sealed.len() - 1;
    sealed[last_idx] ^= 0x01;

    let open_res = EnvelopeCipher::open_with_key(&key, aad, &sealed);
    assert!(matches!(open_res, Err(VaultError::AuthenticationFailed)));
}

#[test]
fn test_envelope_passphrase_roundtrip() {
    let pass = SecretString::new("user-passphrase-xyz");
    let aad = b"channel:55080";
    let plaintext = b"short-lived-symmetric-session-key";

    let sealed = EnvelopeCipher::seal_with_passphrase(&pass, aad, plaintext, KdfParams::fast())
        .expect("seal with passphrase");

    let decrypted = EnvelopeCipher::open_with_passphrase(&pass, aad, &sealed, KdfParams::fast())
        .expect("open with passphrase");

    assert_eq!(decrypted.expose_bytes(), plaintext);
}

#[test]
fn test_native_keyring_fallback() {
    let dir = tempdir().expect("tempdir");
    let vault = KeyringVault::open_native("aqvn-native-test", Some(dir.path().to_path_buf()))
        .expect("open native vault");

    let account = "daemon_id";
    let secret = SecretString::new("aqvn-daemon-session-token");

    vault
        .set_secret(account, &secret)
        .expect("set secret native");
    let loaded = vault
        .get_secret(account)
        .expect("get secret native")
        .expect("loaded");
    assert!(secret.constant_time_eq(&loaded));
}
