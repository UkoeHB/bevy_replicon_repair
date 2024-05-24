//modules
mod common;

//local shortcuts
use bevy_replicon_repair::*;
use common::{BasicComponent, DummyComponent};

//third-party shortcuts
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon::test_app::ServerTestAppExt;

//standard shortcuts


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
            RepliconPlugins.set(bevy_replicon::prelude::ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .replicate_repair::<BasicComponent>();
    }
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: false });

    common::connect(&mut server_app, &mut client_app);

    server_app.world.spawn((Replicated, BasicComponent::default()));

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let _client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>)>()
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
            RepliconPlugins.set(bevy_replicon::prelude::ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .replicate_repair::<BasicComponent>();
    }
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: false });

    // initial connection
    let _client_id = common::connect(&mut server_app, &mut client_app);

    server_app.world.spawn((Replicated, BasicComponent::default()));

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let initial_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 1);

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    // reconnect
    common::reconnect(&mut server_app, &mut client_app);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world.resource::<State<ClientRepairState>>(), ClientRepairState::Done);

    let new_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(new_client_entity, initial_client_entity);
    assert_eq!(client_app.world.entities().len(), 1);
}

//-------------------------------------------------------------------------------------------------------------------

// component mutation during disconnect is replicated into the same entity after a reconnect
#[test]
fn disconnect_component_mutation_travels()
{
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            RepliconPlugins.set(bevy_replicon::prelude::ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .replicate_repair::<BasicComponent>();
    }
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: false });

    // initial connection
    let _client_id = common::connect(&mut server_app, &mut client_app);

    server_app.world.spawn((Replicated, BasicComponent::default()));

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let initial_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 1);

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    // mutate component
    let mut component = server_app
        .world
        .query_filtered::<&mut BasicComponent, With<Replicated>>()
        .single_mut(&mut server_app.world);
    *component = BasicComponent(1);

    // reconnect
    common::reconnect(&mut server_app, &mut client_app);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world.resource::<State<ClientRepairState>>(), ClientRepairState::Done);

    let (new_client_entity, component) = client_app
        .world
        .query_filtered::<(Entity, &BasicComponent), With<Replicated>>()
        .single(&client_app.world);
    assert_eq!(new_client_entity, initial_client_entity);
    assert_eq!(*component, BasicComponent(1));
    assert_eq!(client_app.world.entities().len(), 1);
}

//-------------------------------------------------------------------------------------------------------------------

// component removal during disconnect is mirrored on the replicated entity after a reconnect
#[test]
fn disconnect_component_removal_travels()
{
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            RepliconPlugins.set(bevy_replicon::prelude::ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .replicate_repair::<BasicComponent>();
    }
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: false });

    // initial connection
    let _client_id = common::connect(&mut server_app, &mut client_app);

    server_app.world.spawn((Replicated, BasicComponent::default()));

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let initial_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 1);

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    // remove component
    let server_entity = server_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>)>()
        .single(&server_app.world);
    server_app.world.entity_mut(server_entity).remove::<BasicComponent>();

    // reconnect
    common::reconnect(&mut server_app, &mut client_app);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world.resource::<State<ClientRepairState>>(), ClientRepairState::Done);

    let new_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replicated>, Without<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(new_client_entity, initial_client_entity);
    assert_eq!(client_app.world.entities().len(), 1);
}

//-------------------------------------------------------------------------------------------------------------------

// entity despawn during disconnect is mirrored after a reconnect
#[test]
fn disconnect_despawn_travels()
{
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            RepliconPlugins.set(bevy_replicon::prelude::ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .replicate_repair::<BasicComponent>()
        .replicate_repair::<DummyComponent>();
    }
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: false });

    // initial connection
    let _client_id = common::connect(&mut server_app, &mut client_app);

    server_app.world.spawn((Replicated, BasicComponent::default()));
    //this is needed because replicon won't replicate zero entities, so no init message will be sent on reconnect
    server_app.world.spawn((Replicated, DummyComponent));

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let initial_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 2);

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    // despawn entity
    let server_entity = server_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>)>()
        .single(&server_app.world);
    server_app.world.despawn(server_entity);

    // reconnect
    common::reconnect(&mut server_app, &mut client_app);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world.resource::<State<ClientRepairState>>(), ClientRepairState::Done);

    let dummy_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<DummyComponent>)>()
        .single(&client_app.world);
    assert_ne!(dummy_client_entity, initial_client_entity);
    assert_eq!(client_app.world.entities().len(), 1);
}

//-------------------------------------------------------------------------------------------------------------------

// client entity with retained replicated component is not removed after a reconnect
#[test]
fn retained_component_not_removed()
{
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            RepliconPlugins.set(bevy_replicon::prelude::ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .replicate_repair::<BasicComponent>()
        .replicate_repair::<DummyComponent>();
    }
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: false });

    // initial connection
    let _client_id = common::connect(&mut server_app, &mut client_app);

    server_app.world.spawn((Replicated, BasicComponent::default()));

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let initial_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world);
    assert_eq!(client_app.world.entities().len(), 1);

    client_app.world.entity_mut(initial_client_entity).insert((DummyComponent, Retain::<DummyComponent>::default()));

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    // reconnect
    common::reconnect(&mut server_app, &mut client_app);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world.resource::<State<ClientRepairState>>(), ClientRepairState::Done);

    let final_client_entity = client_app
        .world
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>, With<DummyComponent>)>()
        .single(&client_app.world);
    assert_eq!(final_client_entity, initial_client_entity);
    assert_eq!(client_app.world.entities().len(), 1);
}

//-------------------------------------------------------------------------------------------------------------------
