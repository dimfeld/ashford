Always use `cargo add` to add Rust crate dependencies. This ensures the latest version.

## TypeScript Type Generation (ts-rs)

API-facing types in `ashford-core` use `ts-rs` to generate TypeScript types for the web UI.

When adding or modifying types that are exposed via the API:
1. Add `#[derive(TS)]` and `#[ts(export)]` to the type
2. For `serde_json::Value` fields, use `#[ts(type = "Record<string, unknown>")]`
3. For `i64` fields, use `#[ts(type = "number")]` to avoid bigint serialization issues
4. Run `cargo test --test export_ts_types -- --ignored` to regenerate types

Types are exported to `../web/src/lib/types/generated/`.
