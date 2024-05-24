# Changelog

## [0.7.0]

### Changed

- Update to `bevy_cobweb` v0.6.
- Update to `bevy_replicon` v0.25.


## [0.6.0]

### Changed

- Update to Bevy v0.13


## [0.5.0]

### Changed

- The server and client plugins now panic if not added after the corresponding replicon plugins.

### Fixed

- `bevy_replicon`'s `ParentSync` component is now automatically registered for repair if `ParentSyncPlugin` is present.

### Added

- Added `add_replication_repair` method to the app extension, for use when a component has already been registered for replication and you just need to add repair to it.


## [0.4.0]

### Changed

- Update to `bevy_replicon` v0.21.


## [0.3.0]

### Changed

- Rename: `RepliconRepairPluginClient/Server` -> `ClientPlugin`/`ServerPlugin`.


## [0.2.0]

### Changed

- Rename: `Ignore<C>` -> `Retain<C>`.


## [0.1.0]

- Initial release.
