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

// normal prespawning
#[test]
fn prespawn_normal()
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
    server_app.add_plugins(bevy_replicon_repair::ServerPlugin);
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: true });

    let client_id = common::connect(&mut server_app, &mut client_app);

    let client_entity = client_app.world_mut().spawn(Prespawned).id();
    let server_entity = server_app.world_mut().spawn((Replicated, BasicComponent::default())).id();
    server_app.world_mut().resource_mut::<ClientEntityMap>().insert(client_id, ClientMapping{ server_entity, client_entity });

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let replicated_client_entity = client_app
        .world_mut()
        .query_filtered::<Entity, (With<Prespawned>, With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world());
    assert_eq!(client_app.world().entities().len(), 1);
    assert_eq!(replicated_client_entity, client_entity);
}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity spawned on server and replicated before disconnect survives reconnect
#[test]
fn prespawn_replicated_and_survives()
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
    server_app.add_plugins(bevy_replicon_repair::ServerPlugin);
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: true });

    // initial connection
    let client_id = common::connect(&mut server_app, &mut client_app);

    let client_entity = client_app.world_mut().spawn(Prespawned).id();
    let server_entity = server_app.world_mut().spawn((Replicated, BasicComponent::default())).id();
    server_app.world_mut().resource_mut::<ClientEntityMap>().insert(client_id, ClientMapping{ server_entity, client_entity });

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let replicated_client_entity = client_app
        .world_mut()
        .query_filtered::<Entity, (With<Prespawned>, With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world());
    assert_eq!(client_app.world().entities().len(), 1);
    assert_eq!(replicated_client_entity, client_entity);

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    // reconnect
    common::reconnect(&mut server_app, &mut client_app, client_id);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world().resource::<ClientRepairState>(), ClientRepairState::Done);

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let replicated_client_entity = client_app
        .world_mut()
        .query_filtered::<Entity, (With<Prespawned>, With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world());
    assert_eq!(client_app.world().entities().len(), 1);
    assert_eq!(replicated_client_entity, client_entity);
}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity spawned on server and not replicated before disconnect survives reconnect
#[test]
fn prespawn_not_replicated_and_survives()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    */

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
    server_app.add_plugins(bevy_replicon_repair::ServerPlugin);
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: true });

    // initial connection
    let client_id = common::connect(&mut server_app, &mut client_app);

    let client_entity = client_app.world_mut().spawn(Prespawned).id();
    let server_entity = server_app.world_mut().spawn((Replicated, BasicComponent::default())).id();
    server_app.world_mut().resource_mut::<ClientEntityMap>().insert(client_id, ClientMapping{ server_entity, client_entity });

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    // reconnect
    common::reconnect(&mut server_app, &mut client_app, client_id);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world().resource::<ClientRepairState>(), ClientRepairState::Done);

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    assert_eq!(client_app.world().entities().len(), 1);
    let replicated_client_entity = client_app
        .world_mut()
        //.query_filtered::<Entity, (With<Prespawned>, With<Replicated>, With<BasicComponent>)>()
        .query_filtered::<Entity, (With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world());
    assert_eq!(client_app.world().entities().len(), 1);
    assert_eq!(replicated_client_entity, client_entity);
}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity spawned on server after disconnect survives reconnect
#[test]
fn prespawn_at_disconnect_survives()
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
    server_app.add_plugins(bevy_replicon_repair::ServerPlugin);
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: true });

    // initial connection
    let client_id = common::connect(&mut server_app, &mut client_app);

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    let client_entity = client_app.world_mut().spawn(Prespawned).id();
    let server_entity = server_app.world_mut().spawn((Replicated, BasicComponent::default())).id();
    server_app.world_mut().resource_mut::<ClientEntityMap>().insert(client_id, ClientMapping{ server_entity, client_entity });

    // reconnect
    common::reconnect(&mut server_app, &mut client_app, client_id);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world().resource::<ClientRepairState>(), ClientRepairState::Done);

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let replicated_client_entity = client_app
        .world_mut()
        .query_filtered::<Entity, (With<Prespawned>, With<Replicated>, With<BasicComponent>)>()
        .single(&client_app.world());
    assert_eq!(client_app.world().entities().len(), 1);
    assert_eq!(replicated_client_entity, client_entity);
}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity not spawned on server during disconnect dies after reconnect if cleanup option is set to true
#[test]
fn prespawn_fail_dies_with_cleanup()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    */

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
    server_app.add_plugins(bevy_replicon_repair::ServerPlugin);
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: true });

    // initial connection
    let client_id = common::connect(&mut server_app, &mut client_app);

    let client_entity = client_app.world_mut().spawn(Prespawned).id();
    let _server_entity = server_app.world_mut().spawn((Replicated, DummyComponent)).id();

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    // reconnect
    common::reconnect(&mut server_app, &mut client_app, client_id);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world().resource::<ClientRepairState>(), ClientRepairState::Done);

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let replicated_client_entity = client_app
        .world_mut()
        .query_filtered::<Entity, (Without<Prespawned>, With<Replicated>, With<DummyComponent>)>()
        .single(&client_app.world());
    assert_eq!(client_app.world().entities().len(), 1);
    assert_ne!(replicated_client_entity, client_entity);
}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity not spawned on server during disconnect survives reconnect if cleanup option is set to false
#[test]
fn prespawn_fail_ignored_without_cleanup()
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
    server_app.add_plugins(bevy_replicon_repair::ServerPlugin);
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: false });

    // initial connection
    let client_id = common::connect(&mut server_app, &mut client_app);

    let client_entity = client_app.world_mut().spawn(Prespawned).id();
    let _server_entity = server_app.world_mut().spawn((Replicated, DummyComponent)).id();

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    // reconnect
    common::reconnect(&mut server_app, &mut client_app, client_id);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world().resource::<ClientRepairState>(), ClientRepairState::Done);

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let unreplicated_client_entity = client_app
        .world_mut()
        .query_filtered::<Entity, (With<Prespawned>, Without<Replicated>, Without<BasicComponent>)>()
        .single(&client_app.world());
    let replicated_client_entity = client_app
        .world_mut()
        .query_filtered::<Entity, (Without<Prespawned>, With<Replicated>, With<DummyComponent>)>()
        .single(&client_app.world());
    assert_eq!(client_app.world().entities().len(), 2);
    assert_eq!(unreplicated_client_entity, client_entity);
    assert_ne!(replicated_client_entity, client_entity);
}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity spawned on client while connecting but before init message survives reconnect
#[test]
fn prespawn_while_waiting_survives()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    */

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
    server_app.add_plugins(bevy_replicon_repair::ServerPlugin);
    client_app.add_plugins(bevy_replicon_repair::ClientPlugin{ cleanup_prespawns: true });

    // initial connection
    let client_id = common::connect(&mut server_app, &mut client_app);

    // disconnect
    common::disconnect(&mut server_app, &mut client_app);

    // reconnect
    common::reconnect(&mut server_app, &mut client_app, client_id);
    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world().resource::<ClientRepairState>(), ClientRepairState::Waiting);

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    // spawning on client while waiting for init message
    let client_entity = client_app.world_mut().spawn(Prespawned).id();
    // spawn a replicated entity on server to trigger an init message
    let _server_entity = server_app.world_mut().spawn((Replicated, DummyComponent)).id();

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();
    assert_eq!(*client_app.world().resource::<ClientRepairState>(), ClientRepairState::Done);

    let unreplicated_client_entity = client_app
        .world_mut()
        .query_filtered::<Entity, (With<Prespawned>, Without<Replicated>, Without<DummyComponent>)>()
        .single(&client_app.world());
    assert_eq!(client_app.world().entities().len(), 2);
    assert_eq!(unreplicated_client_entity, client_entity);
}

//-------------------------------------------------------------------------------------------------------------------
