# Bevy Replicon Repair

Adds client state repair to `bevy_replicon` for reconnects.

Use this crate if you want client replication state to persist across a reconnect. Without this crate, you need to manually clean up all `bevy_replicon` replicated entities on a client when the client disconnects, since those entities will be replicated as new entities on reconnect.



## Reconnect handling

This crate is not an all-in-one solution to reconnect handling. Client and server events (direct messages) can fail if sent at or after a disconnect. This means after a disconnect, any client with state tied to server events (or tied to the expectation that client-sent events arrived on the server) may have stale state. Typically you want the server to send 'initialization' messages to a client that has just connected, to transmit anything the client won't receive from replication.

In terms of client architecture, you generally want to trap the client in a loading screen while waiting for all initialization (or reinitialization) state to arrive. You can use change detection on [`RepliconTick`](bevy_replicon::prelude::RepliconTick) to detect when the server's first replication message arrives, and you can manually track the arrival of expected initialization messages.

Note that renet does not support automatic reconnects. To reconnect a disconnected client you need to acquire a completely new connect token from the server/backend then recreate the renet client and transport resources.



## Usage

### Registering components for replication

We wrapped `bevy_replicon`'s component-registration API [`AppReplicationExt`](bevy_replicon::prelude::AppReplicationExt) in our own app extension [`AppReplicationRepairExt`](bevy_replicon_repair::AppReplicationRepairExt).

The wrapped API lets you define a custom 'component-removal function' which will be called on client entities after the first server replication message following a reconnect. That function should remove any replication-registered components that failed to replicate after reconnecting (implying they were removed on the server). We provide a default function [`repair_component`](bevy_replicon_repair::repair_component) that behaves how you would expect.

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


### Client

Clients must include the [`RepliconRepairPluginClient`](bevy_replicon_repair::RepliconRepairPluginClient) plugin.

The client plugin includes a [`cleanup_prespawns`](bevy_replicon_repair::RepliconRepairPluginClient::cleanup_prespawns) option for users of `bevy_replicon`'s client entity pre-mapping functionality. See the [documentation](bevy_replicon_repair::RepliconRepairPluginClient::cleanup_prespawns) for more details.

```rust
fn setup_client(app: &mut App)
{
    setup_replication(app);  //replicate Health
    app.insert_plugins(RepliconRepairPluginClient{ cleanup_prespawns: true });
}
```


### Server

Servers must include the [`RepliconRepairPluginServer`](bevy_replicon_repair::RepliconRepairPluginServer) plugin.

```rust
fn setup_server(app: &mut App)
{
    setup_replication(app);  //replicate Health
    app.insert_plugins(RepliconRepairPluginServer);
}
```



## `bevy_replicon` compatability

| `bevy_replicon` | `bevy_replicon_repair` |
|-------|----------------|
| 0.19  | 0.0.1 - master |
