//local shortcuts

//third-party shortcuts
use bevy::ecs::system::Despawn;
use bevy::prelude::*;
use bevy::utils::EntityHashSet;
use bevy_replicon::{client_just_disconnected, client_connecting, client_just_connected};
use bevy_replicon::prelude::{
    AppReplicationExt, BufferedUpdates, ClientMapper, ClientSet, Ignored, MapNetworkEntities, Replication, RepliconTick,
    ServerEntityMap, ServerEntityTicks,
};
use bevy_replicon::replicon_core::replication_rules::{
    SerializeFn, DeserializeFn, RemoveComponentFn, serialize_component, deserialize_component, remove_component,
    deserialize_mapped_component,
};
use bincode::{DefaultOptions, Options};
use serde::{de::DeserializeOwned, Serialize};

//standard shortcuts
use std::io::Cursor;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Deref, DerefMut)]
struct ComponentRepairRules(Vec<RepairComponentFn>);

impl Default for ComponentRepairRules { fn default() -> Self { Self(Vec::default()) } }

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Default, Deref, DerefMut)]
struct CachedPrespawns(EntityHashSet<Entity>);

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
){
    for entity in replicated.iter()
    {
        commands.add(
            move |world: &mut World|
            {
                let rules = world.remove_resource::<ComponentRepairRules>().unwrap();
                for rule in rules.iter()
                {
                    let Some(mut entity) = world.get_entity_mut(entity) else { return; };
                    (*rule)(&mut entity);
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
/// - After the client state is repaired, `Added` or `Changed` filters will be triggered for replicated components that
///   use the default deserializer, even if a replicated component's value did not change on the server since before
///   the reconnect.
///   If you want to avoid this, use the [`deserialize_eq_component`] and [`deserialize_mapped_eq_component`]
///   deserializers for `Eq` components that doesn't write component data if it won't change.
/// - Since `bevy_replicon` allows you to define custom deserializers for replicated components, we allow you to
///   register custom component-removal systems which will run on all replicated entities during repair.
///   This is a heavy-handed approach, because if a client adds a replicated component to a replicated entity in their
///   own system (e.g. they add `Transform` in reaction to a replicated blueprint, and also register `Transform` as
///   a component that can be replicated), then the component-removal systems may remove it from the entity erroneously.
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
        let cleanup_prespawns = self.cleanup_prespawns;

        if cleanup_prespawns
        {
            app.init_resource::<CachedPrespawns>();
        }

        app.add_state::<ClientRepairState>()
            .init_resource::<ComponentRepairRules>()
            .configure_sets(PreUpdate,
                ClientRepairSet
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<RepliconTick>())
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

/// Signature of component repair functions.
pub type RepairComponentFn = fn(&mut EntityWorldMut);

//-------------------------------------------------------------------------------------------------------------------

/// Default component repair for [`AppReplicationRepairExt`].
///
/// The component `C` will be removed from `entity` if the component was not added/changed on the entity in the repair
/// tick.
///
/// If you manually added/changed the component on the entity in the repair tick, it may be erroneously left alone.
/// Likewise, if you are not replicating the component and instead manually inserted it, it may be erroneously removed.
///
/// You can disable this function for an entity by adding an [`Ignored<C>`](bevy_replicon::prelude::Ignored) component
/// to it.
pub fn repair_component<C: Component>(entity: &mut EntityWorldMut)
{
    let world_tick = unsafe { entity.world_mut().change_tick() };

    // check if the component is ignored from replication
    if entity.contains::<Ignored<C>>() { return; };

    // check if the component exists on the entity
    let Some(change_ticks) = entity.get_change_ticks::<C>() else { return; };

    // check if the component was mutated this tick, indicating it was replicated this tick
    if change_ticks.last_changed_tick() == world_tick { return; }

    entity.remove::<C>();
}

//-------------------------------------------------------------------------------------------------------------------

/// Default deserialization function, with an equality check before writing to the entity.
pub fn deserialize_eq_component<C: Component + DeserializeOwned + Eq>(
    entity         : &mut EntityWorldMut,
    _entity_map    : &mut ServerEntityMap,
    cursor         : &mut Cursor<&[u8]>,
    _replicon_tick : RepliconTick,
) -> bincode::Result<()>
{
    let component: C = DefaultOptions::new().deserialize_from(cursor)?;
    if let Some(existing) = entity.get::<C>()
    {
        if *existing == component { return Ok(()); }
    }
    entity.insert(component);

    Ok(())
}

//-------------------------------------------------------------------------------------------------------------------

/// Like [`deserialize_eq_component`], but also maps entities before insertion.
pub fn deserialize_mapped_eq_component<C: Component + DeserializeOwned + MapNetworkEntities + Eq>(
    entity         : &mut EntityWorldMut,
    entity_map     : &mut ServerEntityMap,
    cursor         : &mut Cursor<&[u8]>,
    _replicon_tick : RepliconTick,
) -> bincode::Result<()>
{
    let mut component: C = DefaultOptions::new().deserialize_from(cursor)?;

    entity.world_scope(|world| {
        component.map_entities(&mut ClientMapper::new(world, entity_map));
    });

    if let Some(existing) = entity.get::<C>()
    {
        if *existing == component { return Ok(()); }
    }

    entity.insert(component);

    Ok(())
}

//-------------------------------------------------------------------------------------------------------------------

pub trait AppReplicationRepairExt
{
    /// Mirrors [`AppReplicationExt::replicate`](bevy_replicon::prelude::AppReplicationExt) using the default
    /// component-removal repair function [`repair_component`].
    fn replicate_repair<C>(&mut self) -> &mut Self
    where
        C: Component + Serialize + DeserializeOwned;

    /// Mirrors [`AppReplicationExt::replicate_mapped`](bevy_replicon::prelude::AppReplicationExt) using the default
    /// component-removal repair function.
    fn replicate_repair_mapped<C>(&mut self) -> &mut Self
    where
        C: Component + Serialize + DeserializeOwned + MapNetworkEntities;

    /// Mirrors [`AppReplicationExt::replicate_with`](bevy_replicon::prelude::AppReplicationExt) with a user-defined
    /// component-removal repair function.
    fn replicate_repair_with<C>(
        &mut self,
        serialize: SerializeFn,
        deserialize: DeserializeFn,
        remove: RemoveComponentFn,
        repair: RepairComponentFn,
    ) -> &mut Self
    where
        C: Component;
}

impl AppReplicationRepairExt for App {
    fn replicate_repair<C>(&mut self) -> &mut Self
    where
        C: Component + Serialize + DeserializeOwned,
    {
        self.replicate_repair_with::<C>(
                serialize_component::<C>,
                deserialize_component::<C>,
                remove_component::<C>,
                repair_component::<C>,
            )
    }

    fn replicate_repair_mapped<C>(&mut self) -> &mut Self
    where
        C: Component + Serialize + DeserializeOwned + MapNetworkEntities,
    {
        self.replicate_repair_with::<C>(
                serialize_component::<C>,
                deserialize_mapped_component::<C>,
                remove_component::<C>,
                repair_component::<C>,
            )
    }

    fn replicate_repair_with<C>(
        &mut self,
        serialize: SerializeFn,
        deserialize: DeserializeFn,
        remove: RemoveComponentFn,
        repair: RepairComponentFn,
    ) -> &mut Self
    where
        C: Component,
    {
        self.replicate_with::<C>(serialize, deserialize, remove);
        self.world.resource_mut::<ComponentRepairRules>().push(repair);

        self
    }
}

//-------------------------------------------------------------------------------------------------------------------
