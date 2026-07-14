# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.22.0] - 2026-07-14

### Changed
- Version bump to 0.22.0 to maintain workspace version synchronization.

## [0.21.0] - 2026-07-10

### Breaking Changes
- `ToolFn` derive: remove the `with_context` attribute flag. Generated `call` implementations now always invoke `handle_impl(params)`. The context-passing variant existed solely to thread MCP roots into handlers, which are no longer used.

## [0.20.1] - 2026-06-20

### Changed
- Version bump to 0.20.1 to maintain workspace version synchronization.

## [0.20.0] - 2026-06-19

### Changed
- Version bump to 0.20.0 to maintain workspace version synchronization

## [0.20.0-rc.1] - 2026-05-24

### Changed
- Version bump to 0.20.0-rc.1 to maintain workspace version synchronization

## [0.18.7] - 2026-03-03

### Changed
- Version bump to 0.18.7 to maintain workspace version synchronization

## [0.18.6] - 2026-02-25

### Changed
- Version bump to 0.18.6 to maintain workspace version synchronization

## [0.18.5] - 2026-02-22

### Changed
- Version bump to 0.18.5 to maintain workspace version synchronization

## [0.18.4] - 2026-02-20

### Changed
- Version bump to 0.18.4 to maintain workspace version synchronization

## [0.18.3] - 2026-02-17

### Changed
- Version bump to 0.18.3 to maintain workspace version synchronization

## [0.18.2] - 2026-02-17

### Changed
- Version bump to 0.18.2 to maintain workspace version synchronization

## [0.18.1] - 2026-02-10

## [0.18.0] - 2026-01-15

### Changed
- Updated dependency to Bevy 0.18.0 stable release

## [0.18.0-rc.1] - 2025-12-21

### Changed
- **Upgraded to Bevy 0.18.0-rc.1**: Updated bevy dependency from 0.17.x to 0.18.0-rc.1

## [0.17.2] - 2025-11-20

### Changed
- Version bump to 0.17.2 to maintain workspace version synchronization

## [0.17.1] - 2025-11-20

### Changed
- Version bump to 0.17.1 to maintain workspace version synchronization

## [0.17.0] - 2025-10-31

### Changed
- Version numbering now tracks Bevy releases for clearer compatibility signaling

### Added
- Initial release of procedural macros for bevy_brp_mcp
- `ToolDescription` derive macro for automatic help text loading from files
