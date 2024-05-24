//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::ecs::component::Tick;
use bevy::ecs::entity::MapEntities;
use bevy::prelude::*;
use bevy_replicon::core::{replication_fns::rule_fns::RuleFns, replication_rules::{AppRuleExt, GroupReplication}};
use serde::{de::DeserializeOwned, Serialize};

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Default component repair for [`AppReplicationRepairExt`].
///
/// The component `C` will be removed from `entity` if the component was not added/changed on the entity in the repair
/// tick.
///
/// If you manually added/changed the component on the entity in the repair tick, it may be erroneously left in place.
/// Likewise, if you are not replicating the component and instead manually inserted it, it may be erroneously removed.
///
/// You can disable this function for a client entity by adding a [`Retain<C>`](crate::Retain) component to it.
pub fn repair_component<C: Component>(entity: &mut EntityWorldMut, preinit_tick: Tick)
{
    let world_tick = unsafe { entity.world_mut().change_tick() };

    // check if the component should be retained
    if entity.contains::<Retain<C>>() { return; };

    // check if the component exists on the entity
    let Some(change_ticks) = entity.get_change_ticks::<C>() else { return; };

    // check if the component was mutated by the most recent replication message
    if change_ticks.is_changed(preinit_tick, world_tick) { return; }

    entity.remove::<C>();
}

//-------------------------------------------------------------------------------------------------------------------

pub trait AppReplicationRepairExt
{
    /// Mirrors [`AppRuleExt::replicate`](bevy_replicon::prelude::AppRuleExt::replicate) using the default
    /// component-removal repair function [`repair_component`].
    fn replicate_repair<C>(&mut self) -> &mut Self
    where
        C: Component + Serialize + DeserializeOwned;

    /// Mirrors [`AppRuleExt::replicate_mapped`](bevy_replicon::prelude::AppRuleExt::replicate_mapped) using
    /// the default component-removal repair function [`repair_component`].
    fn replicate_repair_mapped<C>(&mut self) -> &mut Self
    where
        C: Component + Serialize + DeserializeOwned + MapEntities;

    /// Mirrors [`AppRuleExt::replicate_with`](bevy_replicon::prelude::AppRuleExt::replicate_with) with
    /// a user-defined component-removal repair function.
    fn replicate_repair_with<C>(
        &mut self,
        rules: RuleFns<C>,
        repair: RepairComponentFn,
    ) -> &mut Self
    where
        C: Component;

    /// Mirrors [`AppRuleExt::replicate_group`](bevy_replicon::prelude::AppRuleExt::replicate_group) with
    /// a user-defined component-removal repair function.
    fn replicate_repair_group<C>(
        &mut self,
        repair: RepairComponentFn,
    ) -> &mut Self
    where
        C: GroupReplication;

    /// Registers a user-defined component-removal repair function.
    ///
    /// This can be used for components that were already registered for replication via `bevy_replicon`'s API.
    fn add_replication_repair_fn(
        &mut self,
        repair: RepairComponentFn,
    ) -> &mut Self;
}

impl AppReplicationRepairExt for App {
    fn replicate_repair<C>(&mut self) -> &mut Self
    where
        C: Component + Serialize + DeserializeOwned,
    {
        self.replicate_repair_with::<C>(
                RuleFns::default(),
                repair_component::<C>,
            )
    }

    fn replicate_repair_mapped<C>(&mut self) -> &mut Self
    where
        C: Component + Serialize + DeserializeOwned + MapEntities,
    {
        self.replicate_repair_with::<C>(
                RuleFns::default_mapped(),
                repair_component::<C>,
            )
    }

    fn replicate_repair_with<C>(
        &mut self,
        rules: RuleFns<C>,
        repair: RepairComponentFn,
    ) -> &mut Self
    where
        C: Component,
    {
        self.replicate_with::<C>(rules);
        self.add_replication_repair_fn(repair);

        self
    }

    fn replicate_repair_group<C>(
        &mut self,
        repair: RepairComponentFn,
    ) -> &mut Self
    where
        C: GroupReplication
    {
        self.replicate_group::<C>();
        self.add_replication_repair_fn(repair);

        self
    }

    fn add_replication_repair_fn(
        &mut self,
        repair: RepairComponentFn,
    ) -> &mut Self
    {
        if !self.world.contains_resource::<ComponentRepairRules>()
        { self.world.init_resource::<ComponentRepairRules>(); }

        self.world.resource_mut::<ComponentRepairRules>().push(repair);

        self
    }
}

//-------------------------------------------------------------------------------------------------------------------
