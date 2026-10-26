# Spec delta: change14-benchmarks-readme

## ADDED Requirements

### Requirement: change14-benchmarks-readme binds mission feature to openspec change
The bridge script `opsx-mission-bind.mjs` MUST scaffold a 1:1 openspec change for mission feature `benchmarks-readme`, with `mission_lineage` populated and a `task_inventory[]` derived from `expectedBehavior[]`.

#### Scenario: change14-benchmarks-readme-benchmark-sh-runs-all-5-gates-prints-tab
- GIVEN the bridge script is invoked with `--feature-id benchmarks-readme`
- WHEN the change scaffold is produced
- THEN the behavior "benchmark.sh runs all 5 gates, prints table, correct exit code, no orphans" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change14-benchmarks-readme-real-measured-numbers-recorded-all-gates
- GIVEN the bridge script is invoked with `--feature-id benchmarks-readme`
- WHEN the change scaffold is produced
- THEN the behavior "Real measured numbers recorded; all gates PASS (or honest failure report to orchestrator)" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.

#### Scenario: change14-benchmarks-readme-readme-complete-for-all-platforms-securi
- GIVEN the bridge script is invoked with `--feature-id benchmarks-readme`
- WHEN the change scaffold is produced
- THEN the behavior "README complete for all platforms + security model + limitations; whitelabel grep clean" is represented in `task_inventory[]`
- AND the corresponding VAL-* assertion is referenced in `test_strategy`.
