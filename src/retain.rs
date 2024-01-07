//local shortcuts

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::marker::PhantomData;

//-------------------------------------------------------------------------------------------------------------------

/// Marker component for client entities that prevents component removal during reconnect repair.
///
/// See [`repair_component`](crate::repair_component).
#[derive(Component)]
pub struct Retain<T>(PhantomData<T>);

impl<T> Default for Retain<T> { fn default() -> Self { Self(PhantomData::default()) } }

//-------------------------------------------------------------------------------------------------------------------
