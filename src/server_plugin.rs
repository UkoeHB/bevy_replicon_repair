//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::ecs::component::Tick;
use bevy::ecs::system::Despawn;
use bevy::prelude::*;
use bevy::utils::EntityHashSet;
use bevy_replicon::{client_just_disconnected, client_connecting, client_just_connected};
use bevy_replicon::prelude::{
    BufferedUpdates, ClientSet, Replication, RepliconTick,
    ServerEntityMap, ServerEntityTicks,
};

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
            if cached.insert(mapping.server_entity, (client_id, mapping.client_entity)).is_some()
            { tracing::error!(client_id, ?mapping, "overwriting cached client mapping"); }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn copy_client_map(mut events: EventReader<ServerEvent>, mut mapped: ResMut<ClientEntityMap>, cached: Res<CachedClientMap>)
{
    for event in events.read()
    {
        match event
        {
            ServerEvent::ClientConnected{ client_id } =>
            {
                for (server_entity, (mapped_client_id, client_entity)) in cached.iter()
                {
                    if client_id != mapped_client_id { continue; }
                    mapped.entry(client_id)
                        .or_default()
                        .push(ClientMapping{ server_entity, client_entity });
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
            tracing::trace!(client_id, server_entity, client_entity,
                "removing despawned server entity from cached client-entity map");
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(SystemSet, Debug, Default, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ServerRepairSet;

#[derive(Debug)]
pub struct RepliconRepairPluginServer;

impl Plugin for RepliconRepairPluginServer
{
    fn build(&self, app: &mut App)
    {
        app.add_state::<ClientRepairState>()
            .init_resource::<CachedClientMap>()
            .configure_sets(PreUpdate,
                ServerRepairSet
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<RepliconTick>())
            )
            .add_systems(PreUpdate,
                collect_world_change_tick
                    .before(ClientSet::Receive)
                    .run_if(not(in_state(ClientRepairState::Dormant)))
            )
            .add_systems(PreUpdate,
                (
                    // state: -> Disconnected
                    (
                        clear_prespawn_cache.run_if(move || cleanup_prespawns),
                        clear_buffered_updates,
                        detect_just_disconnected,
                        apply_state_transition::<ClientRepairState>,
                    )
                        .chain()
                        .run_if(client_just_disconnected()),
                    // state: Disconnected -> Waiting
                    (
                        detect_waiting,
                        apply_state_transition::<ClientRepairState>,
                    )
                        .chain()
                        .run_if(client_connecting().or_else(client_just_connected()))
                        .run_if(in_state(ClientRepairState::Disconnected)),
                    // state: Waiting -> Repairing
                    (
                        detect_first_replication,
                        apply_state_transition::<ClientRepairState>,
                    )
                        .chain()
                        .run_if(in_state(ClientRepairState::Waiting))
                        .run_if(resource_changed::<RepliconTick>()),
                    // repair
                    // state: Repairing -> Done
                    (
                        despawn_missing_entities,
                        apply_deferred,
                        (
                            despawn_failed_prespawns,
                            clear_prespawn_cache,
                            apply_deferred,
                        )
                            .chain()
                            .run_if(move || cleanup_prespawns),
                        cleanup_entity_components,
                        apply_deferred,
                        finish_repair,
                        apply_state_transition::<ClientRepairState>,
                    )
                        .chain()
                        .run_if(in_state(ClientRepairState::Repairing))
                )
                    .chain()
                    .in_set(ServerRepairSet)
            )
            .add_systems(Last,
                collect_prespawns
                    .run_if(move || cleanup_prespawns)
                    .run_if(in_state(ClientRepairState::Waiting))
                    .in_set(ServerRepairSet)
            );
    }
}

//-------------------------------------------------------------------------------------------------------------------
