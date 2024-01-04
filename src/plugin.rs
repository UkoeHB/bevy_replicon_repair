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

#[derive(Resource, Default, Deref, DerefMut)]
struct CachedPrespawns(EntityHashSet<Entity>);

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Deref, DerefMut)]
struct RepairChangeTickTracker(Tick);

impl Default for RepairChangeTickTracker { fn default() -> Self { Self(Tick::new(0)) } }

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn collect_prespawns(mut cached_prespawns: ResMut<CachedPrespawns>, prespawns: Query<Entity, Added<Prespawned>>)
{
    for prespawn in prespawns.iter()
    {
        let _ = cached_prespawns.insert(prespawn);
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn collect_world_change_tick(world: &mut World)
{
    let world_tick = world.change_tick();
    **world.resource_mut::<RepairChangeTickTracker>() = world_tick;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn detect_just_disconnected(mut next: ResMut<NextState<ClientRepairState>>)
{
    next.set(ClientRepairState::Disconnected);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn detect_waiting(mut next: ResMut<NextState<ClientRepairState>>)
{
    next.set(ClientRepairState::Waiting);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn detect_first_replication(mut next: ResMut<NextState<ClientRepairState>>)
{
    next.set(ClientRepairState::Repairing);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn finish_repair(mut next: ResMut<NextState<ClientRepairState>>)
{
    next.set(ClientRepairState::Done);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Iterate client entity map after first init message, despawn entities with old replicon tick + remove from map.
fn despawn_missing_entities(
    mut commands   : Commands,
    mut tick_map   : ResMut<ServerEntityTicks>,
    mut entity_map : ResMut<ServerEntityMap>,
    replicon_tick  : Res<RepliconTick>,
){
    let replicon_tick = *replicon_tick;
    tick_map.retain(
            |entity, tick|
            {
                if *tick == replicon_tick { return true; }
                commands.add(Despawn{ entity: *entity });
                entity_map.remove_by_client(*entity);
                false
            }
        );
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn clear_prespawn_cache(mut cached: ResMut<CachedPrespawns>)
{
    cached.clear();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn clear_buffered_updates(mut buffered: ResMut<BufferedUpdates>)
{
    buffered.clear();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Iterate prespawned entities, despawn if not in client entity map and not prespawned since this connection session
/// started.
fn despawn_failed_prespawns(
    mut commands : Commands,
    cached       : Res<CachedPrespawns>,
    tick_map     : Res<ServerEntityTicks>,
    prespawned   : Query<Entity, With<Prespawned>>,
){
    for entity in prespawned.iter()
    {
        if cached.contains(&entity) { continue; }
        if tick_map.contains_key(&entity) { continue; }
        commands.add(Despawn{ entity });
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

//todo: this could be more efficient...
fn cleanup_entity_components(
    mut commands : Commands,
    replicated   : Query<Entity, With<Replication>>,
    preinit_tick : Res<RepairChangeTickTracker>,
){
    let preinit_tick = **preinit_tick;
    for entity in replicated.iter()
    {
        commands.add(
            move |world: &mut World|
            {
                let rules = world.remove_resource::<ComponentRepairRules>().unwrap();
                for rule in rules.iter()
                {
                    let Some(mut entity) = world.get_entity_mut(entity) else { return; };
                    (*rule)(&mut entity, preinit_tick);
                }
                world.insert_resource(rules);
            }
        );
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Tracks the sequence of events leading up to replication repair.
///
/// The state will only leave `Dormant` after the first client disconnect. This ensures repair will not
/// run needlessly for the first connection session.
#[derive(States, Default, Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum ClientRepairState
{
    /// The client is in its initial connection session.
    #[default]
    Dormant,
    /// The client is disconnected.
    Disconnected,
    /// The client is connecting or connected but has not yet received its first replication message.
    Waiting,
    /// Handling the first replication message.
    Repairing,
    /// The first replication message has been handled.
    Done,
}

//-------------------------------------------------------------------------------------------------------------------

/// Marker component for entities prespawned on a client that are expected to be replicated by the server.
///
/// This component should be added to all prespawned entities that you want to be auto-cleaned up by
/// [`RepliconClientRepairPlugin`] after a reconnect.
#[derive(Component, Debug, Default, Copy, Clone)]
pub struct Prespawned;

//-------------------------------------------------------------------------------------------------------------------

#[derive(SystemSet, Debug, Default, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ClientRepairSet;

/// Adds client repair functionality to a client app that uses `bevy_replicon`.
/// - Despawns replicated entities that fail to re-replicate after a reconnect.
/// - Despawns [`Prespawned`] entities that fail to replicate after a reconnect (optional).
/// - Runs custom component-removal systems on replicated entities after a reconnect.
///
/// The goal of this plugin is to streamline client reconnects as much as possible by preserving existing client
/// entities. There are a couple points to keep in mind:
/// - After the client state is repaired, `Changed` filters will be triggered for replicated components that
///   use the default deserializer, even if a replicated component's value did not change on the server since before
///   the reconnect.
///   We use change detection to remove dead replicated components, so with the current design this is unavoidable.
/// - Since `bevy_replicon` allows you to define custom deserializers for replicated components, we allow you to
///   register custom component-removal systems which will run on all replicated entities during repair.
///   This is a heavy-handed approach, because if a client adds a replicated component to a replicated entity in their
///   own system (e.g. they add `Transform` in reaction to a replicated blueprint, and also register `Transform` as
///   a component that can be replicated), then the component-removal systems may remove it from the entity erroneously.
///   See [`repair_component`] for how to selectively disable it and avoid that problem.
#[derive(Debug)]
pub struct RepliconClientRepairPlugin
{
    /// If true, client entities with the [`Prespawned`] component will be despawned if the server does not replicate
    /// them in the first replication message after a reconnect. This is used for cleaning up client entities that are
    /// pre-mapped on the server.
    ///
    /// A client entity will be despawned only if it was spawned **before** the current client connection session started,
    /// even if it fails to replicate in the first server replication message.
    /// This is because entities prespawned in the current session may have successfully landed on the server but not
    /// yet been replicated (due to a race condition between client-sent events and the server's first replication
    /// message).
    /// - **Caveat**: You should only spawn [`Prespawned`] entities after your system that initializes/reinitializes your
    ///   renet client.
    ///   Entities spawned before that system will be considered 'spawned in the current
    ///   session' even if spawned when renet is disconnected.
    ///   As a result, we won't despawn them if they fail to be replicated in the first server replication
    ///   message.
    ///   Also, if you spawn entities in schedule `Last`, do so before the [`ClientRepairSet`] otherwise we
    ///   won't track them for cleanup.
    ///
    /// Note that in general it is possible for a server to reject a client request to spawn an entity. This means
    /// users typically need their own tracking and cleanup systems for failed prespawns. Users that want
    /// to use their own cleanup systems instead of ours should set this to `false`.
    pub cleanup_prespawns: bool,
}

impl Plugin for RepliconClientRepairPlugin
{
    fn build(&self, app: &mut App)
    {
        // disable replicon's cleanup
        app.configure_sets(PreUpdate, ClientSet::Reset.run_if(|| false));

        // set up repair cleanup
        let cleanup_prespawns = self.cleanup_prespawns;

        if cleanup_prespawns
        {
            app.init_resource::<CachedPrespawns>();
        }

        app.add_state::<ClientRepairState>()
            .init_resource::<ComponentRepairRules>()
            .init_resource::<RepairChangeTickTracker>()
            .configure_sets(PreUpdate,
                ClientRepairSet
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
                    .in_set(ClientRepairSet)
            )
            .add_systems(Last,
                collect_prespawns
                    .run_if(move || cleanup_prespawns)
                    .run_if(in_state(ClientRepairState::Waiting))
                    .in_set(ClientRepairSet)
            );
    }
}

//-------------------------------------------------------------------------------------------------------------------
