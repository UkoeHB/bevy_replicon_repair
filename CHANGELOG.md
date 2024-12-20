# Changelog

## [0.10.0]

- Update to `bevy_replicon` v0.28.1, `bevy_cobweb` v0.12.


## [0.9.0]

- Update to `bevy` v0.14, `bevy_replicon` v0.27.


## [0.8.3]

### Fixed

- Run `ClientRepairSet` between `ClientSet::Receive` and `ClientSet::SyncHierarchy`.


## [0.8.2]

### Fixed

- Clients now use `despawn_recursive` when cleaning up entities.


## [0.8.1]

### Fixed

- `ServerRepairSet` now runs after `ServerSet::StoreHierarchy` and also runs in `PreUpdate` as it did before.


## [0.8.0]

### Changed

- Update to `bevy_replicon` v0.26.


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
