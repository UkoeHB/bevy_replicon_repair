//modules
//mod common;

//local shortcuts
use bevy_replicon_repair::*;

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

// prespawned entity spawned on server before disconnect survives reconnect
#[test]
fn prespawn_survives()
{

}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity spawned on server after disconnect survives reconnect
#[test]
fn prespawn_at_disconnect_survives()
{

}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity not spawned on server during disconnect dies after reconnect
#[test]
fn prespawn_fail_dies()
{

}

//-------------------------------------------------------------------------------------------------------------------

// prespawned entity spawned on client while connecting but before init message survives reconnect
#[test]
fn prespawn_while_waiting_survives()
{

}

//-------------------------------------------------------------------------------------------------------------------
