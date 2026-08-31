# Changelog

## [v0.1.0] — 2026-08-30

### Added
- Initial implementation — build-time embed of the host zoneinfo database
  (TZif-only, `right/`/`posix/` skipped, content-deduplicated,
  deflate-compressed) with `list`, `install`, `set`, `cat`, and `version`
  commands.

### Fixed
- Test asserted wrong subtree-match semantics (a pattern selects itself and
  its subtree).

### Documentation
- README, project CLAUDE.md with design and work plan.
- Recorded measured sizes: 463 KB binary embedding all 599 zones of tzdata
  2025b vs 4.2 MB zoneinfo tree; verified byte-identical output on dev.g8.lo.

## [Unreleased]
<!-- New unreleased changes go here -->
