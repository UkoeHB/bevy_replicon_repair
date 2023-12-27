//modules
mod common;

//local shortcuts
use bevy_replicon_repair::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy_renet::renet::ClientId;
use bevy_replicon::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

#[derive(Component, Eq, PartialEq, Serialize, Deserialize)]
struct DummyComponent;

//-------------------------------------------------------------------------------------------------------------------

// normal replication works with new app extension
#[test]
fn normal_replication()
{
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            ReplicationPlugins.set(ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
            RepliconClientRepairPlugin{
                cleanup_prespawns: false,
            },
        ))
        .replicate_repair::<DummyComponent>();
    }

    common::connect(&mut server_app, &mut client_app);

    server_app.world.spawn((Replication, DummyComponent));

    server_app.update();
    std::thread::sleep(std::time::Duration::from_millis(50));
    client_app.update();

    let _client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replication>, With<DummyComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 1);
}

//-------------------------------------------------------------------------------------------------------------------

// entity stays alive after reconnect, no new entity spawned
#[test]
fn entity_persists()
{
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            ReplicationPlugins.set(ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
            RepliconClientRepairPlugin{
                cleanup_prespawns: false,
            },
        ))
        .replicate_repair::<DummyComponent>();
    }

    // initial connection
    let (client_id, server_port) = common::connect(&mut server_app, &mut client_app);

    server_app.world.spawn((Replication, DummyComponent));

    server_app.update();
    std::thread::sleep(std::time::Duration::from_millis(50));
    client_app.update();

    let initial_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replication>, With<DummyComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 1);

    // disconnect
    client_app.world.resource_mut::<RenetClient>().disconnect();
    client_app.update();
    std::thread::sleep(std::time::Duration::from_millis(50));
    server_app.update();
    assert!(!server_app.world.resource::<RenetServer>().is_connected(ClientId::from_raw(client_id)));

    // reconnect
    common::reconnect(&mut server_app, &mut client_app, client_id, server_port);
    assert_eq!(*client_app.world.resource::<State<ClientRepairState>>(), ClientRepairState::Done);

    let new_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replication>, With<DummyComponent>)>()
        .single(&client_app.world);
    assert_eq!(new_client_entity, initial_client_entity);
    assert_eq!(client_app.world.entities().len(), 1);
}

//-------------------------------------------------------------------------------------------------------------------

// component mutation during disconnect is replicated into the same entity after a reconnect
#[test]
fn disconnect_component_mutation_travels()
{

}

//-------------------------------------------------------------------------------------------------------------------

// component removal during disconnect is repaired on the replicated entity after a reconnect
#[test]
fn disconenct_component_removal_travels()
{

}

//-------------------------------------------------------------------------------------------------------------------

// entity despawn during disconnect is repaired after a reconnect
#[test]
fn disconnect_despawn_travels()
{

}

//-------------------------------------------------------------------------------------------------------------------

// Eq deserializer prevents change detection from being triggered on value-equal mutation.
#[test]
fn eq_component_ignored()
{

}

//-------------------------------------------------------------------------------------------------------------------
