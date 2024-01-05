//local shortcuts

//third-party shortcuts
use bevy::ecs::component::Tick;
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Signature of component repair functions.
///
/// We pass in a world change tick from before the first server init message for the current session.
/// This can be used to detect component changes caused by replication, which indicates a component was replicated.
///
/// See [`repair_component`](crate::repair_component) for the default implementation.
pub type RepairComponentFn = fn(&mut EntityWorldMut, Tick);

//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct ComponentRepairRules(Vec<RepairComponentFn>);

impl Default for ComponentRepairRules { fn default() -> Self { Self(Vec::default()) } }

//-------------------------------------------------------------------------------------------------------------------
