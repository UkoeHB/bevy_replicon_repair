# Bevy Replicon Repair

This crate extends [`bevy_replicon`](https://github.com/lifescapegame/bevy_replicon) with client reconnect handling so that client replication state will persist across a reconnect.

Without this crate, you need to manually clean up all `bevy_replicon` replicated entities on a client when the client disconnects, since those entities will be replicated as new entities on reconnect.



## Reconnect handling

This crate is not an all-in-one solution to reconnect handling. Client and server events (direct messages) can fail if sent at or after a disconnect. This means after a disconnect, any client with state tied to server events (or tied to the expectation that client-sent events arrived on the server) may have stale state. Typically you want the server to send 'initialization' messages to a client that has just connected, to transmit anything the client won't receive from replication.

In terms of client architecture, you generally want to trap the client in a loading screen while waiting for all initialization (or reinitialization) state to arrive, so the user can't interact with an incomplete world. You can use change detection on [`ServerInitTick`](bevy_replicon::client::ServerInitTick) to detect when the server's first replication message arrives, and you can manually track the arrival of expected initialization messages.

Note that renet does not support automatic reconnects. To reconnect a client you need to acquire a completely new connect token from the server/backend then recreate the renet client and transport resources.



## Usage

### Registering components for replication

We wrapped `bevy_replicon`'s component-registration API [`AppRuleExt`](bevy_replicon::prelude::AppRuleExt) in our own app extension [`AppReplicationRepairExt`](bevy_replicon_repair::AppReplicationRepairExt).

The wrapped API lets you define a custom 'component-removal function' which will be called on client entities after the first server replication message following a reconnect. That function should remove any replication-registered components that failed to replicate after reconnecting. We provide a default function [`repair_component`](bevy_replicon_repair::repair_component) that behaves how you would expect.

Here is an example using default component-registration:

```rust
#[derive(Component)]
struct Health(usize);

fn setup_replication(app: &mut App)
{
    // bevy_replicon equivalent: `app.replicate::<Health>();`
    app.replicate_repair::<Health>();
}
```

Note that if you have a component that was already registered with `bevy_replicon`'s API, you can add replication repair with [`add_replication_repair_fn`](bevy_replicon_repair::AppReplicationRepairExt::add_replication_repair_fn).

The `bevy_replicon` component `ParentSync` is registered for repair by default if `ParentSyncPlugin` is present.


### Client

Clients must include the [`ClientPlugin`](bevy_replicon_repair::ClientPlugin).

The client plugin includes a [`cleanup_prespawns`](bevy_replicon_repair::ClientPlugin::cleanup_prespawns) option for users of `bevy_replicon`'s client entity pre-mapping functionality. See the [documentation](bevy_replicon_repair::ClientPlugin::cleanup_prespawns) for more details.

```rust
fn setup_client(app: &mut App)
{
    setup_replication(app);  //replicate Health
    app.insert_plugins(ClientPlugin{ cleanup_prespawns: true });
}
```


### Server

Servers must include the [`ServerPlugin`](bevy_replicon_repair::ServerPlugin).

```rust
fn setup_server(app: &mut App)
{
    setup_replication(app);  //replicate Health
    app.insert_plugins(ServerPlugin);
}
```



## `bevy_replicon` compatability

| `bevy_replicon` | `bevy_replicon_repair` |
|-------|----------------|
| 0.25  | 0.7 - master   |
| 0.23  | 0.5            |
| 0.21  | 0.4            |
| 0.19  | 0.1 - 0.3      |
