# Changelog

## [0.2.0] - 2026-04-04

### Added
- `BillingProvider` trait for unified provider abstraction (`providers::traits`)
- Shared aggregation module (`providers::aggregation`)
- Provider factory for static dispatch (`providers::factory`)
- `RawBillItem` normalized struct for cross-provider bill items
- Billing adapter structs for all 7 providers
- Unit tests for aggregation (empty, single, multiple products, regions, instance count)
- Unit tests for `BillingData` serialization and `ProductCost::default()`
- Modular TUI architecture (views/widgets/styles)
- ASCII logo, loading splash screen, status indicators
- `scripts/install.sh` for one-line installation
- Rustdoc comments for all public API items

### Changed
- Rewrote `service.rs` from 1258 lines to ~340 lines using trait-based dispatch
- Extracted duplicated ProductAggregator logic into `providers::aggregation::aggregate_bill_items`
- Individual `query_xxx()` methods are now thin wrappers around `query_provider()`
- TUI refactored from single 800-line file to modular components
- README rewritten with asciinema demo, dynamic install URLs
- All comments translated to English

### Fixed
- Clippy `const_is_empty` warning
- Install script permission issue (now defaults to `~/.local/bin`)

## [0.1.0] - 2026-03-22

### Added
- Initial release
- Support for 7 cloud providers: Aliyun, AWS, Tencent Cloud, Volcengine, UCloud, GCP, Cloudflare
- TUI dashboard with provider tabs, bar chart, product table
- CLI commands: query, providers, summary
- CSV export
- Multi-account JSON configuration
- Feature flags for modular compilation
