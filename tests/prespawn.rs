//modules
mod common;

//local shortcuts
use bevy_replicon_repair::*;
use common::{BasicComponent, DummyComponent};

//third-party shortcuts
use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use bevy_replicon::*;
use bevy_replicon::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

// normal prespawning
#[test]
fn prespawn_normal()
{
    let mut server_app = App::new();
    let mut client_app = App::new();
    server_app.add_plugins(RepliconRepairPluginServer);
    client_app.add_plugins(RepliconRepairPluginClient{ cleanup_prespawns: true });
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            ReplicationPlugins.set(ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .replicate_repair::<BasicComponent>();
    }

    let (client_id, _server_port) = common::connect(&mut server_app, &mut client_app);
    let client_id = ClientId::from_raw(client_id);

    let client_entity = client_app.world.spawn(Prespawned).id();
    let server_entity = server_app.world.spawn((Replication, BasicComponent::default())).id();
    server_app.world.resource_mut::<ClientEntityMap>().insert(client_id, ClientMapping{ server_entity, client_entity });

    server_app.update();
    std::thread::sleep(std::time::Duration::from_millis(50));
    client_app.update();

    let replicated_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Prespawned>, With<Replication>, With<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 1);
    assert_eq!(replicated_client_entity, client_entity);
}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity spawned on server and replicated before disconnect survives reconnect
#[test]
fn prespawn_replicated_and_survives()
{
    let mut server_app = App::new();
    let mut client_app = App::new();
    server_app.add_plugins(RepliconRepairPluginServer);
    client_app.add_plugins(RepliconRepairPluginClient{ cleanup_prespawns: true });
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            ReplicationPlugins.set(ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .replicate_repair::<BasicComponent>();
    }

    // initial connection
    let (client_id, server_port) = common::connect(&mut server_app, &mut client_app);
    let client_id = ClientId::from_raw(client_id);

    let client_entity = client_app.world.spawn(Prespawned).id();
    let server_entity = server_app.world.spawn((Replication, BasicComponent::default())).id();
    server_app.world.resource_mut::<ClientEntityMap>().insert(client_id, ClientMapping{ server_entity, client_entity });

    server_app.update();
    std::thread::sleep(std::time::Duration::from_millis(50));
    client_app.update();

    let replicated_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Prespawned>, With<Replication>, With<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 1);
    assert_eq!(replicated_client_entity, client_entity);

    // disconnect
    client_app.world.resource_mut::<RenetClient>().disconnect();
    client_app.update();
    std::thread::sleep(std::time::Duration::from_millis(50));
    server_app.update();
    assert!(!server_app.world.resource::<RenetServer>().is_connected(client_id));

    // reconnect
    common::reconnect(&mut server_app, &mut client_app, client_id.raw(), server_port);
    assert_eq!(*client_app.world.resource::<State<ClientRepairState>>(), ClientRepairState::Done);

    server_app.update();
    std::thread::sleep(std::time::Duration::from_millis(50));
    client_app.update();

    let replicated_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Prespawned>, With<Replication>, With<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 1);
    assert_eq!(replicated_client_entity, client_entity);
}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity spawned on server and not replicated before disconnect survives reconnect
#[test]
fn prespawn_not_replicated_and_survives()
{
    let mut server_app = App::new();
    let mut client_app = App::new();
    server_app.add_plugins(RepliconRepairPluginServer);
    client_app.add_plugins(RepliconRepairPluginClient{ cleanup_prespawns: true });
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            ReplicationPlugins.set(ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .replicate_repair::<BasicComponent>();
    }

    // initial connection
    let (client_id, server_port) = common::connect(&mut server_app, &mut client_app);
    let client_id = ClientId::from_raw(client_id);

    let client_entity = client_app.world.spawn(Prespawned).id();
    let server_entity = server_app.world.spawn((Replication, BasicComponent::default())).id();
    server_app.world.resource_mut::<ClientEntityMap>().insert(client_id, ClientMapping{ server_entity, client_entity });

    // disconnect
    client_app.world.resource_mut::<RenetClient>().disconnect();
    client_app.update();
    std::thread::sleep(std::time::Duration::from_millis(75));
    server_app.update();
    assert!(!server_app.world.resource::<RenetServer>().is_connected(client_id));

    // reconnect
    common::reconnect(&mut server_app, &mut client_app, client_id.raw(), server_port);
    assert_eq!(*client_app.world.resource::<State<ClientRepairState>>(), ClientRepairState::Done);

    server_app.update();
    std::thread::sleep(std::time::Duration::from_millis(50));
    client_app.update();

    let replicated_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Prespawned>, With<Replication>, With<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 1);
    assert_eq!(replicated_client_entity, client_entity);
}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity spawned on server after disconnect survives reconnect
#[test]
fn prespawn_at_disconnect_survives()
{

}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity not spawned on server during disconnect dies after reconnect
#[test]
fn prespawn_fail_dies()
{

}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity spawned on client while connecting but before init message survives reconnect
#[test]
fn prespawn_while_waiting_survives()
{

}

//-------------------------------------------------------------------------------------------------------------------
