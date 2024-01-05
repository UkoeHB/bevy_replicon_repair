//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::utils::EntityHashMap;
use bevy_replicon::RenetReceive;
use bevy_replicon::renet::{ClientId, ServerEvent};
use bevy_replicon::prelude::{ClientEntityMap, ClientMapping, Replication, RepliconTick, ServerSet};

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// [ server entity : (client id : client entity) ]
#[derive(Resource, Default, Deref, DerefMut)]
struct CachedClientMap(EntityHashMap<Entity, (ClientId, Entity)>);

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn collect_client_map(mut cached: ResMut<CachedClientMap>, mapped: Res<ClientEntityMap>)
{
    for (client_id, mappings) in mapped.iter()
    {
        for mapping in mappings.iter()
        {
            // only one server <-> client entity mapping is currently supported per server entity
            if cached.insert(mapping.server_entity, (*client_id, mapping.client_entity)).is_some()
            { tracing::error!(?client_id, ?mapping, "overwriting cached client mapping"); }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn return_client_map(
    mut events : EventReader<ServerEvent>,
    mut mapped : ResMut<ClientEntityMap>,
    cached     : Res<CachedClientMap>
){
    for event in events.read()
    {
        match event
        {
            ServerEvent::ClientConnected{ client_id } =>
            {
                for (server_entity, (mapped_client_id, client_entity)) in cached.iter()
                {
                    if client_id != mapped_client_id { continue; }
                    mapped.insert(*client_id, ClientMapping{ server_entity: *server_entity, client_entity: *client_entity });
                }
            }
            _ => (),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn clean_client_map(mut cached: ResMut<CachedClientMap>, mut despawns: RemovedComponents<Replication>)
{
    for server_entity in despawns.read()
    {
        if let Some((client_id, client_entity)) = cached.remove(&server_entity)
        {
            tracing::trace!(?client_id, ?server_entity, ?client_entity,
                "removing despawned server entity from cached client-entity map");
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(SystemSet, Debug, Default, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ServerRepairSet;

/// Adds client repair functionality to a server app that uses `bevy_replicon`.
/// - Preserves client entity mappings for disconnected clients.
///   This is most useful for repairing entity mappings when a client prespawn notification is applied on the server
///   but then the client disconnects before it can receive the replicated server entity.
///   Since the client won't have the mapping, we need to link the server entity to the client entity after the client
///   reconnects so the client doesn't end up with a dangling prespawned entity.
#[derive(Debug)]
pub struct RepliconRepairPluginServer;

impl Plugin for RepliconRepairPluginServer
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<CachedClientMap>()
            .init_resource::<ComponentRepairRules>()
            .configure_sets(PreUpdate,
                ServerRepairSet
                    .after(RenetReceive)
                    .run_if(resource_exists::<RepliconTick>())
            )
            .add_systems(PreUpdate,
                (
                    // collect the current map before replicon cleans it in response to disconnects
                    collect_client_map.before(ServerSet::Receive),
                    // clean immediately before repairing the client map to avoid missing despawns
                    clean_client_map,
                    // return existing client mappings as soon as a client connection is detected
                    // - We do this after the replicon receive set in case a disconnect and connect event show up
                    //   in the same tick (this would be a bug, but we want to be defensive here).
                    return_client_map.after(ServerSet::Receive),
                )
                    .chain()
                    .in_set(ServerRepairSet)
            );
    }
}

//-------------------------------------------------------------------------------------------------------------------
