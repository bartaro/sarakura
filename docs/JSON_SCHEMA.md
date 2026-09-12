# SARAKURA JSON Schemas

`sarakura schema export --out schemas` writes the v1.0 schema bundle used by CI, KOKURA, and KUROSAKI integration.

Required v1.0 files include:

- `sarakura_ai_diagnostics.schema.json`
- `sarakura_diagnostic_summary.schema.json`
- `sarakura_retest_plan.schema.json`
- `sarakura_repair_plan.schema.json`
- `sarakura_automation_plan.schema.json`
- `sarakura_baseline_delta.schema.json`
- `sarakura_ci_summary.schema.json`
- `sarakura_catalog_coverage.schema.json`
- `sarakura_diagnostic_pack_plan.schema.json`
- `sarakura_repro_bundle_manifest.schema.json`

Strict validation is intentionally implemented in Rust, not only JSON Schema, because v1.0 needs cross-field checks such as duplicate `diagnostic_id`, confidence bounds, non-empty `repair_target.target_type`, and non-empty `retest_condition.expect_absent`.

