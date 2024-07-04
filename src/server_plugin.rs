//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::ecs::entity::EntityHashMap;
use bevy_replicon::prelude::*;
use bevy_replicon::server::server_tick::ServerTick;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// [ server entity : (client id : client entity) ]
#[derive(Resource, Default, Deref, DerefMut)]
struct CachedClientMap(EntityHashMap<(ClientId, Entity)>);

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn collect_client_map(mut cached: ResMut<CachedClientMap>, mapped: Res<ClientEntityMap>)
{
    for (client_id, mappings) in mapped.iter()
    {
        for mapping in mappings.iter()
        {
            // only one server <-> client entity mapping is currently supported per server entity
            // - Note: This warning will print if `ClientEntityMap` entries are inserted before `ServerSet::Receive`.
            if cached.insert(mapping.server_entity, (*client_id, mapping.client_entity)).is_some()
            { tracing::warn!(?client_id, ?mapping, "overwriting cached client mapping"); }
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

fn clean_client_map(mut cached: ResMut<CachedClientMap>, mut despawns: RemovedComponents<Replicated>)
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

/// System set in [`PostUpdate`] for server repair. Runs before [`ServerSet::Send`].
#[derive(SystemSet, Debug, Default, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ServerRepairSet;

/// Adds client repair functionality to a server app that uses `bevy_replicon`.
/// - Preserves client entity mappings for disconnected clients.
///   This is most useful for repairing entity mappings when a client prespawn notification is applied on the server
///   but then the client disconnects before it can receive the replicated server entity.
///   Since the client won't have the mapping, we need to link the server entity to the client entity after the client
///   reconnects so the client doesn't end up with a dangling prespawned entity.
///
/// Note that if [`Replicated`] is removed from a mapped server entity and reinserted, then the mapping will not be
/// sent in the next reconnect.
/// This may be a source of bugs, so be careful.
///
/// This plugin must be added after `bevy_replicon`'s `ClientPlugin`.
#[derive(Debug)]
pub struct ServerPlugin;

impl Plugin for ServerPlugin
{
    fn build(&self, app: &mut App)
    {
        if !app.is_plugin_added::<bevy_replicon::prelude::ServerPlugin>()
        { panic!("repair's ServerPlugin depends on replicon's ServerPlugin"); }

        if !app.world().contains_resource::<ComponentRepairRules>()
        { app.world_mut().init_resource::<ComponentRepairRules>(); }

        app.init_resource::<CachedClientMap>()
            .configure_sets(PreUpdate,
                ServerRepairSet
                    .after(ServerSet::ReceivePackets)
                    .before(ServerSet::Receive)
                    .run_if(resource_exists::<ServerTick>)
            )
            .configure_sets(PostUpdate,
                ServerRepairSet
                    .after(ServerSet::StoreHierarchy)
                    .before(ServerSet::Send)
                    .run_if(resource_exists::<ServerTick>)
            )
            .add_systems(PreUpdate,
                (
                    // collect the current map before it gets cleaned up due to a disconnect
                    // - This is mainly needed for unit tests where mappings are inserted manually.
                    collect_client_map,
                )
                    .in_set(ServerRepairSet)
            )
            .add_systems(PostUpdate,
                (
                    // collect the current map
                    collect_client_map,
                    // clean immediately before repairing the client map to avoid missing despawns
                    // - We assume the server does not remove and re-add Replicated to client-mapped server entities.
                    clean_client_map,
                    // return existing client mappings as soon as a client connection is detected
                    return_client_map,
                )
                    .chain()
                    .in_set(ServerRepairSet)
            );
    }
}

//-------------------------------------------------------------------------------------------------------------------
