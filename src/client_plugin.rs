//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::ecs::component::Tick;
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;
use bevy_cobweb::prelude::*;
use bevy_replicon::client::confirm_history::ConfirmHistory;
use bevy_replicon::client::{BufferedMutations, ServerUpdateTick};
use bevy_replicon::core::server_entity_map::ServerEntityMap;
use bevy_replicon::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Prespawned entities that were spawned between when a reconnect attempt started and when the reconnect succeeded.
/// We don't despawn those entities in case they successfully landed on the server.
#[derive(Resource, Default, Deref, DerefMut)]
struct CachedPrespawns(EntityHashSet);

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Deref, DerefMut)]
struct RepairChangeTickTracker(Tick);

impl Default for RepairChangeTickTracker { fn default() -> Self { Self(Tick::new(0)) } }

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Collects entities prespawned after starting to reconnect, in order to despawn entities spawned before that point.
fn collect_prespawns_impl(
    In(collect)          : In<bool>,
    mut cached_prespawns : ResMut<CachedPrespawns>,
    prespawns            : Query<Entity, Added<Prespawned>>
){
    // Since the `Added` filter only works for components added since the last time a system ran, we need to run the
    // system once when attempting to reconnect to initialize the query state to start fresh at that point in time.
    if !collect { return; }

    for prespawn in prespawns.iter()
    {
        let _ = cached_prespawns.insert(prespawn);
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn ignore_added_prespawns(world: &mut World)
{
    syscall(world, false, collect_prespawns_impl);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn collect_prespawns(world: &mut World)
{
    syscall(world, true, collect_prespawns_impl);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn clean_dead_prespawns(mut cached_prespawns: ResMut<CachedPrespawns>, mut despawns: RemovedComponents<Prespawned>)
{
    for prespawn in despawns.read()
    {
        let _ = cached_prespawns.remove(&prespawn);
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

fn initiate_just_disconnected(mut state: ResMut<ClientRepairState>)
{
    if state.in_state(ClientRepairState::Disconnected) { return; }
    state.set(ClientRepairState::Disconnected);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn initiate_waiting(mut state: ResMut<ClientRepairState>)
{
    if state.in_state(ClientRepairState::Waiting) { return; }
    state.set(ClientRepairState::Waiting);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn initiate_repairing(mut state: ResMut<ClientRepairState>)
{
    if state.in_state(ClientRepairState::Repairing) { return; }
    state.set(ClientRepairState::Repairing);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn finish_repair(mut state: ResMut<ClientRepairState>)
{
    if state.in_state(ClientRepairState::Done) { return; }
    state.set(ClientRepairState::Done);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Iterate replicated entities after first init message, despawn entities with old replicon tick + remove from map.
fn despawn_missing_entities(
    mut commands   : Commands,
    replicated     : Query<(Entity, &ConfirmHistory), With<Replicated>>,
    mut entity_map : ResMut<ServerEntityMap>,
    replicon_tick  : Res<ServerUpdateTick>,
){
    for (entity, history) in replicated.iter()
    {
        if history.last_tick() == **replicon_tick { continue; }
        commands.get_entity(entity).map(DespawnRecursiveExt::despawn_recursive);
        entity_map.remove_by_client(entity);
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn clear_prespawn_cache(mut cached: ResMut<CachedPrespawns>)
{
    cached.clear();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn clear_buffered_updates(mut buffered: ResMut<BufferedMutations>)
{
    buffered.clear();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Iterate prespawned entities, despawn if not replicated and not prespawned since this connection session
/// started.
fn despawn_failed_prespawns(
    mut commands : Commands,
    cached       : Res<CachedPrespawns>,
    prespawned   : Query<(Entity, Has<Replicated>), With<Prespawned>>,
){
    for (entity, is_replicated) in prespawned.iter()
    {
        if is_replicated { continue; }
        if cached.contains(&entity) { continue; }
        commands.get_entity(entity).map(DespawnRecursiveExt::despawn_recursive);
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

//todo: this could be more efficient...
fn cleanup_entity_components(
    mut commands : Commands,
    replicated   : Query<Entity, With<Replicated>>,
    preinit_tick : Res<RepairChangeTickTracker>,
){
    let preinit_tick = **preinit_tick;
    for entity in replicated.iter()
    {
        commands.queue(
            move |world: &mut World|
            {
                let rules = world.remove_resource::<ComponentRepairRules>().unwrap();
                for rule in rules.iter()
                {
                    let Ok(mut entity) = world.get_entity_mut(entity) else { return; };
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
#[derive(Resource, Default, Debug, Hash, Eq, PartialEq, Copy, Clone)]
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

impl ClientRepairState
{
    /// Sets the current state.
    pub fn set(&mut self, state: Self)
    {
        *self = state;
    }

    /// Returns `true` if `other` equals `self`.
    pub fn in_state(&self, other: Self) -> bool
    {
        *self == other
    }

    /// Returns `true` if `other` does not equal `self`.
    pub fn not_in_state(&self, other: Self) -> bool
    {
        !self.in_state(other)
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Marker component for entities prespawned on a client that are expected to be replicated by the server.
///
/// This component should be added to all prespawned entities that you want to be auto-cleaned up by
/// [`ClientPlugin`] after a reconnect if the server fails to replicate them.
#[derive(Component, Debug, Default, Copy, Clone)]
pub struct Prespawned;

//-------------------------------------------------------------------------------------------------------------------

/// System set in [`PreUpdate`] that contains all repair systems.
///
/// Runs after [`ClientSet::Receive`].
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
///   We remove dead replicated components by leveraging change detection, so with the current design triggering `Changed`
///   is unavoidable.
/// - We allow you to register custom component-removal systems which will run on all replicated entities during repair.
///   This is a heavy-handed approach, because if a client adds a replicated component to a replicated entity in their
///   own system (e.g. they add `Transform` in reaction to a replicated blueprint, and also register `Transform` as
///   a component that can be replicated), then the component-removal systems may remove it from the entity erroneously.
///   See [`repair_component`] for how to selectively disable component removal.
///
/// The `bevy_replicon` type [`ParentSync`] is automatically registered for repair if [`ParentSyncPlugin`] is present.
///
/// This plugin must be added after `bevy_replicon`'s [`ClientPlugin`](bevy_replicon::prelude::ClientPlugin).
#[derive(Debug)]
pub struct ClientPlugin
{
    /// This is used for cleaning up client entities that are pre-mapped on the server.
    /// You must add a [`Prespawned`] component to all pre-mapped client entities.
    ///
    /// Note that in general it is possible for a server to reject a client request to spawn an entity. This means
    /// users typically need their own tracking and cleanup systems for failed prespawns. Users that want
    /// to use their own cleanup systems instead of ours should set this to `false`.
    ///
    /// ### Details
    ///
    /// Client entities with the [`Prespawned`] component will be despawned if the server does not replicate
    /// them in the first replication message after a reconnect.
    /// We assume any client entity that meets that condition either failed to be spawned on the server (e.g. because the
    /// client message with that entity failed to reach the server due to a disconnect), or was despawned on the server
    /// while the client was disconnected.
    ///
    /// A client entity will be despawned only if it was spawned **before** the current client connection session started,
    /// even if it fails to replicate in the first server replication message.
    /// This is because entities prespawned in the current session may have successfully landed on the server but not
    /// yet been replicated (due to a race condition between client-sent events and the server's first replication
    /// message).
    /// - You should only spawn [`Prespawned`] entities after your system that initializes/reinitializes your
    ///   renet client.
    ///   Entities spawned before that system will be considered 'spawned in the current
    ///   session' even if the client mappings were sent to a dead renet client.
    ///   As a result, we won't despawn them if they fail to be replicated in the first server replication
    ///   message.
    ///   For the best results, reinitialize your renet client between [`ClientSet::ReceivePackets`]
    ///   and [`ClientSet::Receive`] (in `PreUpdate`), and spawn prespawned
    ///   entities after [`ClientRepairSet`] (which also runs in `PreUpdate`).
    /// - If you spawn entities in schedule `Last`, do so before the [`ClientRepairSet`] otherwise we
    ///   won't track them for cleanup.
    pub cleanup_prespawns: bool,
}

impl Plugin for ClientPlugin
{
    fn build(&self, app: &mut App)
    {
        if !app.is_plugin_added::<bevy_replicon::prelude::ClientPlugin>()
        { panic!("repair's ClientPlugin depends on replicon's ClientPlugin"); }

        // disable replicon's cleanup
        app.configure_sets(PreUpdate, ClientSet::Reset.run_if(|| false));

        // pre-register replicon's ParentSync
        if app.is_plugin_added::<ParentSyncPlugin>()
        { app.add_replication_repair_fn(repair_component::<ParentSync>); }

        // set up repair cleanup
        let cleanup_prespawns = self.cleanup_prespawns;

        if cleanup_prespawns
        {
            app.init_resource::<CachedPrespawns>();
        }

        if !app.world().contains_resource::<ComponentRepairRules>()
        { app.world_mut().init_resource::<ComponentRepairRules>(); }

        app.init_resource::<ClientRepairState>()
            .init_resource::<RepairChangeTickTracker>()
            .configure_sets(PreUpdate,
                ClientRepairSet
                    .after(ClientSet::Receive)
                    .before(ClientSet::SyncHierarchy)
                    .run_if(resource_exists::<ServerUpdateTick>)
            )
            .add_systems(PreUpdate,
                collect_world_change_tick
                    .after(ClientSet::ReceivePackets)
                    .before(ClientSet::Receive)
                    .run_if(|s: Res<ClientRepairState>| s.not_in_state(ClientRepairState::Dormant))
            )
            .add_systems(PreUpdate,
                (
                    // state: -> Disconnected
                    (
                        clear_buffered_updates,
                        initiate_just_disconnected,
                    )
                        .chain()
                        .run_if(client_just_disconnected),
                    // state: Disconnected -> Waiting
                    (
                        (
                            clear_prespawn_cache,
                            ignore_added_prespawns,
                        )
                            .chain()
                            .run_if(move || cleanup_prespawns),
                        initiate_waiting,
                    )
                        .chain()
                        .run_if(client_just_connected.or(client_connecting))
                        .run_if(|s: Res<ClientRepairState>| s.in_state(ClientRepairState::Disconnected)),
                    // state: Waiting -> Repairing
                    (
                        initiate_repairing,
                    )
                        .chain()
                        .run_if(|s: Res<ClientRepairState>| s.in_state(ClientRepairState::Waiting))
                        .run_if(resource_changed::<ServerUpdateTick>),
                    // repair
                    // state: Repairing -> Done
                    (
                        despawn_missing_entities,
                        (
                            collect_prespawns,  //we need to collect prespawns from this tick
                            despawn_failed_prespawns,
                            clear_prespawn_cache,
                        )
                            .chain()
                            .run_if(move || cleanup_prespawns),
                        cleanup_entity_components,
                        finish_repair,
                    )
                        .chain()
                        .run_if(|s: Res<ClientRepairState>| s.in_state(ClientRepairState::Repairing)),
                )
                    .chain()
                    .in_set(ClientRepairSet)
            )
            .add_systems(Last,
                (
                    clean_dead_prespawns,  //do this first in case of Prespawned being removed then re-added
                    collect_prespawns,
                )
                    .chain()
                    .run_if(move || cleanup_prespawns)
                    .run_if(|s: Res<ClientRepairState>| s.in_state(ClientRepairState::Waiting))
                    .in_set(ClientRepairSet)
            );
    }
}

//-------------------------------------------------------------------------------------------------------------------
