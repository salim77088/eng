//! ECS wrapper around `hecs`. We add a small set of convenience
//! helpers (spawn-with-name, query-by-tag, deferred command buffer)
//! so callers don't have to juggle `hecs`' raw API directly.

use hecs::{Bundle, Entity, World as HecsWorld};
use parking_lot::RwLock;
use std::sync::Arc;

pub use hecs::{Component, ComponentRef, Query, QueryOne, Ref, RefMut};

/// A named tag - lets scripts and the editor refer to entities by name.
#[derive(Clone, Debug, Default)]
pub struct Name(pub String);

/// Tag - cheap marker component, e.g. `Tag("player")`.
#[derive(Clone, Debug, Default)]
pub struct Tag(pub String);

/// Thin wrapper around `hecs::World` with a deferred command buffer so
/// scripts running during the simulation step can safely spawn/despawn
/// entities without invalidating active queries.
#[derive(Default)]
pub struct World {
    pub(crate) inner: HecsWorld,
    pub(crate) pending: Arc<RwLock<Vec<Command>>>,
}

#[derive(Debug)]
enum Command {
    Despawn(Entity),
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn an entity from a bundle of components. Sugar for
    /// `world.inner.spawn(bundle)` that also queues any deferred work.
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        self.inner.spawn(bundle)
    }

    /// Spawn with a name - sugar for the common case. Returns the new
    /// entity id.
    pub fn spawn_named(&mut self, name: impl Into<String>) -> Entity {
        self.inner.spawn((Name(name.into()),))
    }

    pub fn despawn(&mut self, entity: Entity) {
        // Defer despawn so we don't invalidate queries mid-iteration.
        self.pending.write().push(Command::Despawn(entity));
    }

    /// Flush deferred commands. Call once per frame, outside of any query.
    pub fn flush(&mut self) {
        let cmds: Vec<Command> = std::mem::take(&mut *self.pending.write());
        for cmd in cmds {
            match cmd {
                Command::Despawn(e) => {
                    let _ = self.inner.despawn(e);
                }
            }
        }
    }

    /// Get a live reference into the underlying `hecs::World` for ad-hoc
    /// queries. Be careful not to hold borrows across spawn/despawn.
    pub fn raw(&self) -> &HecsWorld {
        &self.inner
    }

    pub fn raw_mut(&mut self) -> &mut HecsWorld {
        &mut self.inner
    }

    pub fn entity_count(&self) -> usize {
        self.inner.len() as usize
    }
}
