use ahash::AHashMap;
use std::{hash::Hash, rc::Rc};
use thunderdome::Arena;

use crate::Vertex;

use super::{PathBuilder, PathBuilderInner, Tessellator};

const INIT_POINTS_SIZE: usize = 64;
const INIT_VERTS_SIZE: usize = 128;
const INIT_POOL_CAPACITY: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Path(thunderdome::Index);

struct PathCacheBuilderKey {
    builder: Rc<PathBuilderInner>,
}

impl PartialEq for PathCacheBuilderKey {
    fn eq(&self, other: &Self) -> bool {
        &*self.builder == &*other.builder
    }
}

impl Eq for PathCacheBuilderKey {}

impl Hash for PathCacheBuilderKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let builder: &PathBuilderInner = &*self.builder;
        builder.hash(state);
    }
}

struct PathCacheBuilderEntry {
    used: bool,
    id: Path,
}

struct PathCacheEntry {
    used: bool,
    verts: Vec<Vertex>,
    antialiased: bool,
}

// TODO: An option to use a concurrent cache
pub(crate) struct PathCache {
    builder_to_id_map: AHashMap<PathCacheBuilderKey, PathCacheBuilderEntry>,
    paths: thunderdome::Arena<PathCacheEntry>,
}

impl PathCache {
    pub fn new() -> Self {
        Self {
            builder_to_id_map: AHashMap::with_capacity(INIT_POOL_CAPACITY),
            paths: Arena::with_capacity(INIT_POOL_CAPACITY),
        }
    }

    pub fn begin_frame(&mut self) {
        for entry in self.builder_to_id_map.values_mut() {
            entry.used = false;
        }
        for (_, entry) in self.paths.iter_mut() {
            entry.used = false;
        }
    }

    pub fn end_frame(&mut self) {
        self.builder_to_id_map.retain(|_, entry| entry.used);
        self.paths.retain(|_, entry| entry.used);
    }

    pub fn build_cached(
        &mut self,
        path_builder: PathBuilder,
        tess: &mut Tessellator,
        antialias: bool,
    ) -> Path {
        let antialias = antialias && path_builder.inner.antialias;
        let mut path_id = Path(thunderdome::Index::DANGLING);

        let path_builder = Rc::new(path_builder.inner);

        let builder_entry = self.builder_to_id_map.entry(PathCacheBuilderKey {
            builder: Rc::clone(&path_builder),
        });
        builder_entry
            .and_modify(|entry| {
                entry.used = true;

                if let Some(path_entry) = self.paths.get_mut(entry.id.0) {
                    path_entry.used = true;
                } else {
                    let mut verts = Vec::with_capacity(INIT_VERTS_SIZE);
                    tess.tessellate(path_builder.iter_commands(), &mut verts, antialias);

                    entry.id = Path(self.paths.insert(PathCacheEntry {
                        used: true,
                        verts,
                        antialiased: antialias,
                    }));
                }

                path_id = entry.id;
            })
            .or_insert_with(|| {
                let mut verts = Vec::with_capacity(INIT_VERTS_SIZE);
                tess.tessellate(path_builder.iter_commands(), &mut verts, antialias);

                path_id = Path(self.paths.insert(PathCacheEntry {
                    used: true,
                    verts,
                    antialiased: antialias,
                }));

                PathCacheBuilderEntry {
                    used: true,
                    id: path_id,
                }
            });

        path_id
    }

    pub fn build_uncached(
        &mut self,
        path_builder: PathBuilder,
        old_verts: Option<Vec<Vertex>>,
        tess: &mut Tessellator,
        antialias: bool,
    ) -> Path {
        let antialias = antialias && path_builder.inner.antialias;

        let mut verts = old_verts.unwrap_or(Vec::with_capacity(INIT_VERTS_SIZE));
        tess.tessellate(path_builder.inner.iter_commands(), &mut verts, antialias);

        Path(self.paths.insert(PathCacheEntry {
            used: true,
            verts,
            antialiased: antialias,
        }))
    }

    pub fn build_raw(&mut self, verts: Vec<Vertex>, antialiased: bool) -> Path {
        Path(self.paths.insert(PathCacheEntry {
            used: true,
            verts,
            antialiased,
        }))
    }
}
