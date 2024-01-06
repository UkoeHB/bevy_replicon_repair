//local shortcuts

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::marker::PhantomData;

//-------------------------------------------------------------------------------------------------------------------

/// Marker component that can be added to client entities to prevent component removal during reconnect repair.
///
/// See [`repair_component`](crate::repair_component).
#[derive(Component)]
pub struct Ignore<T>(PhantomData<T>);

impl<T> Default for Ignore<T> { fn default() -> Self { Self(PhantomData::default()) } }

//-------------------------------------------------------------------------------------------------------------------
