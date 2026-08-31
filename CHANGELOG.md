# Changelog

## [Unreleased]

### 2026-08-30
- **feat:** Initial implementation — build-time embed of the host zoneinfo
  database (TZif-only, `right/`/`posix/` skipped, content-deduplicated,
  deflate-compressed) with `list`, `install`, `set`, `cat`, and `version`
  commands.
- **docs:** README, project CLAUDE.md with design and work plan.
- **fix:** Test asserted wrong subtree-match semantics (a pattern selects
  itself and its subtree).
- **docs:** Recorded measured sizes: 463 KB binary embedding all 599 zones
  of tzdata 2025b vs 4.2 MB zoneinfo tree; verified byte-identical output
  on dev.g8.lo.
