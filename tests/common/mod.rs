//local shortcuts

//third-party shortcuts
use bevy::prelude::*;
use bevy_replicon::{prelude::*, test_app::ServerTestAppExt};
use serde::{Deserialize, Serialize};

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

#[derive(Component, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(super) struct DummyComponent;

#[derive(Component, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(super) struct BasicComponent(pub(super) usize);

//-------------------------------------------------------------------------------------------------------------------

pub(super) fn connect(server_app: &mut App, client_app: &mut App) -> ClientId
{
    server_app.connect_client(client_app);
    client_app.world.resource::<RepliconClient>().id().unwrap()
}

//-------------------------------------------------------------------------------------------------------------------

pub(super) fn disconnect(server_app: &mut App, client_app: &mut App)
{
    server_app.disconnect_client(client_app);
}

//-------------------------------------------------------------------------------------------------------------------

pub(super) fn reconnect(server_app: &mut App, client_app: &mut App, client_id: ClientId)
{
    let mut client = client_app.world.resource_mut::<RepliconClient>();
    assert!(
        client.is_disconnected(),
        "client can't be connected multiple times"
    );

    client.set_status(RepliconClientStatus::Connected {
        client_id: Some(client_id),
    });

    let mut server = server_app.world.resource_mut::<RepliconServer>();
    server.set_running(true);

    server_app.world.send_event(ServerEvent::ClientConnected { client_id });

    server_app.update(); // Will update `ConnectedClients`, otherwise next call will assign the same ID.
    client_app.update();
}

//-------------------------------------------------------------------------------------------------------------------
