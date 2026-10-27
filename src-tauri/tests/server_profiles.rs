use aqvn_workspace_lib::config::{AppConfig, DEFAULT_SERVER_PROFILES, DEFAULT_SERVER_URL};

#[test]
fn test_server_profiles_default_and_roundtrip() {
    let conf = AppConfig::default();
    assert_eq!(DEFAULT_SERVER_PROFILES, &[DEFAULT_SERVER_URL]);
    assert_eq!(conf.server_profiles, vec![DEFAULT_SERVER_URL]);

    let custom = AppConfig {
        server_url: "https://custom.example.com".to_string(),
        server_profiles: vec![
            "https://a0.matmagoc.com".to_string(),
            "https://custom.example.com".to_string(),
        ],
        ..Default::default()
    };
    let json = serde_json::to_string(&custom).expect("serialize");
    let parsed: AppConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.server_profiles.len(), 2);
    assert_eq!(parsed.server_profiles[1], "https://custom.example.com");
}
