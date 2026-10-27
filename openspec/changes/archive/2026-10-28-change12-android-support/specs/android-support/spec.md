# Spec delta: change12-android-support

## ADDED Requirements

### Requirement: change12-android-support binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `android-support`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change12-android-support-gen-android-committed-and-clean-apk-buil
- GIVEN the bridge script is invoked with `--feature-id android-support`
- WHEN the change scaffold is produced
- THEN the behavior "gen/android committed and clean; APK builds exit 0 with arm64 libs" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change12-android-support-manifest-net-aqvn-workspace-aqvn-label-i
- GIVEN the bridge script is invoked with `--feature-id android-support`
- WHEN the change scaffold is produced
- THEN the behavior "Manifest: net.aqvn.workspace + AQVN label + INTERNET-only" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change12-android-support-rfc-ssh-tray-hotkey-absent-from-android-
- GIVEN the bridge script is invoked with `--feature-id android-support`
- WHEN the change scaffold is produced
- THEN the behavior "RFC/SSH/tray/hotkey absent from Android build (source gating + .so symbol check)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change12-android-support-deferral-of-runtime-test-explicitly-logg
- GIVEN the bridge script is invoked with `--feature-id android-support`
- WHEN the change scaffold is produced
- THEN the behavior "Deferral of runtime test explicitly logged" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
