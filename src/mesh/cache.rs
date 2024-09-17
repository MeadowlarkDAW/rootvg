use ahash::AHashMap;
use std::{hash::Hash, ops::Range};
use thunderdome::Arena;

use crate::{
    paint::{MeshOpts, MAX_STROKE_WIDTH},
    LineCap, LineJoin, Vertex,
};

use super::{commands::PackedCommandBuffer, Tessellator};

const INIT_VERTS_CAPACITY: usize = 128;
const INIT_POOL_CAPACITY: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshID(thunderdome::Index);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CachedMeshID(thunderdome::Index);

impl Into<MeshID> for CachedMeshID {
    fn into(self) -> MeshID {
        MeshID(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UncachedMeshID(thunderdome::Index);

impl UncachedMeshID {
    pub const fn new() -> Self {
        Self(thunderdome::Index::DANGLING)
    }
}

impl Default for UncachedMeshID {
    fn default() -> Self {
        Self::new()
    }
}

impl Into<MeshID> for UncachedMeshID {
    fn into(self) -> MeshID {
        MeshID(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawMeshID(thunderdome::Index);

impl RawMeshID {
    pub const fn new() -> Self {
        Self(thunderdome::Index::DANGLING)
    }
}

impl Default for RawMeshID {
    fn default() -> Self {
        Self::new()
    }
}

impl Into<MeshID> for RawMeshID {
    fn into(self) -> MeshID {
        MeshID(self.0)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct MeshCacheKey {
    pub command_buffer: PackedCommandBuffer,
    pub stroke_width_bytes: [u8; 4],
    pub miter_limit_bytes: [u8; 4],
    pub line_join: LineJoin,
    pub line_cap: LineCap,
    pub fill: bool,
    pub antialias: bool,
}

impl MeshCacheKey {
    pub fn new() -> Self {
        Self {
            command_buffer: PackedCommandBuffer::new(),
            stroke_width_bytes: [0; 4],
            miter_limit_bytes: [0; 4],
            line_join: LineJoin::default(),
            line_cap: LineCap::default(),
            fill: false,
            antialias: false,
        }
    }

    pub fn reset(&mut self) {
        self.command_buffer.clear();
    }

    pub fn set_mesh_opts(&mut self, opts: &MeshOpts, antialiasing_enabled: bool) {
        self.stroke_width_bytes = f32::to_ne_bytes(opts.stroke_width.clamp(0.0, MAX_STROKE_WIDTH));
        self.miter_limit_bytes = f32::to_ne_bytes(opts.miter_limit.max(0.0));
        self.line_join = opts.line_join;
        self.line_cap = opts.line_cap;
        self.fill = opts.fill;
        self.antialias = antialiasing_enabled && opts.anti_alias;
    }
}

pub(crate) struct MeshCacheEntry {
    pub stroke_verts: Vec<Vertex>,
    pub fill_verts: Vec<Vertex>,
    pub stroke_width: f32,
    pub antialiased: bool,
    pub fill_vert_range: Option<Range<u32>>,
    pub stroke_vert_range: Option<Range<u32>>,
}

// TODO: An option to use a concurrent cache that is shared across
// RootVG contexts.
pub(crate) struct MeshCache {
    builder_to_id_map: AHashMap<MeshCacheKey, thunderdome::Index>,
    meshes: thunderdome::Arena<MeshCacheEntry>,
    // TODO: Use a better allocation strategy that tries to keep vertex
    // buffers closer together in memory?
    vert_pool: Vec<Vec<Vertex>>,
    force_rebuild: bool,
}

impl MeshCache {
    pub fn new() -> Self {
        Self {
            builder_to_id_map: AHashMap::with_capacity(INIT_POOL_CAPACITY),
            meshes: Arena::with_capacity(INIT_POOL_CAPACITY),
            vert_pool: Vec::with_capacity(INIT_POOL_CAPACITY * 2),
            force_rebuild: false,
        }
    }

    pub fn begin_frame(&mut self, force_rebuild: bool) {
        self.force_rebuild = force_rebuild;

        if force_rebuild {
            for (_, mesh_id) in self.builder_to_id_map.drain() {
                if let Some(entry) = self.meshes.remove(mesh_id) {
                    // Reuse the allocation of the vertex buffers for next time.
                    // (If the vertex buffer is particuarly long, then don't
                    // reuse it to prevent memory from blowing up.)
                    if !entry.fill_verts.is_empty() && entry.fill_verts.len() <= INIT_VERTS_CAPACITY
                    {
                        self.vert_pool.push(entry.fill_verts);
                    }

                    if !entry.stroke_verts.is_empty()
                        && entry.stroke_verts.len() <= INIT_VERTS_CAPACITY
                    {
                        self.vert_pool.push(entry.stroke_verts);
                    }
                }
            }
        }

        for (_, entry) in self.meshes.iter_mut() {
            entry.fill_vert_range = None;
            entry.stroke_vert_range = None;
        }
    }

    pub fn end_frame(&mut self) {
        self.meshes.retain(|_, entry| {
            if entry.fill_vert_range.is_some() || entry.stroke_vert_range.is_some() {
                true
            } else {
                // Reuse the allocation of the vertex buffers for next time.
                // (If the vertex buffer is particuarly long, then don't
                // reuse it to prevent memory from blowing up.)
                if !entry.fill_verts.is_empty() && entry.fill_verts.len() <= INIT_VERTS_CAPACITY {
                    let mut tmp = Vec::new();
                    std::mem::swap(&mut tmp, &mut entry.fill_verts);
                    tmp.clear();
                    self.vert_pool.push(tmp);
                }

                if !entry.stroke_verts.is_empty() && entry.stroke_verts.len() <= INIT_VERTS_CAPACITY
                {
                    let mut tmp = Vec::new();
                    std::mem::swap(&mut tmp, &mut entry.stroke_verts);
                    tmp.clear();
                    self.vert_pool.push(tmp);
                }

                false
            }
        });

        self.builder_to_id_map
            .retain(|_, mesh_id| self.meshes.contains(*mesh_id));
    }

    pub fn build_mesh_cached(
        &mut self,
        tess: &mut Tessellator,
        key: &MeshCacheKey,
    ) -> CachedMeshID {
        let mut build = || -> thunderdome::Index {
            let stroke_width = f32::from_ne_bytes(key.stroke_width_bytes);

            let mut fill_verts = if key.fill {
                self.vert_pool
                    .pop()
                    .unwrap_or_else(|| Vec::with_capacity(INIT_VERTS_CAPACITY))
            } else {
                Vec::new()
            };
            let mut stroke_verts = if stroke_width > 0.0 {
                self.vert_pool
                    .pop()
                    .unwrap_or_else(|| Vec::with_capacity(INIT_VERTS_CAPACITY))
            } else {
                Vec::new()
            };

            tess.tessellate(key, &mut stroke_verts, &mut fill_verts);

            self.meshes.insert(MeshCacheEntry {
                fill_vert_range: None,
                stroke_vert_range: None,
                stroke_verts,
                fill_verts,
                stroke_width,
                antialiased: key.antialias,
            })
        };

        if let Some(mesh_id) = self.builder_to_id_map.get_mut(key) {
            if self.force_rebuild {
                *mesh_id = build();
            }

            CachedMeshID(*mesh_id)
        } else {
            let mesh_id = build();

            self.builder_to_id_map.insert(key.clone(), mesh_id);

            CachedMeshID(mesh_id)
        }
    }

    pub fn build_mesh_uncached(
        &mut self,
        tess: &mut Tessellator,
        key: &MeshCacheKey,
        mesh_id: &mut UncachedMeshID,
    ) -> UncachedMeshID {
        if let Some(entry) = &mut self.meshes.get_mut(mesh_id.0) {
            entry.stroke_width = f32::from_ne_bytes(key.stroke_width_bytes);
            entry.antialiased = key.antialias;

            tess.tessellate(key, &mut entry.stroke_verts, &mut entry.fill_verts);
        } else {
            let stroke_width = f32::from_ne_bytes(key.stroke_width_bytes);

            let mut fill_verts = if key.fill {
                self.vert_pool
                    .pop()
                    .unwrap_or_else(|| Vec::with_capacity(INIT_VERTS_CAPACITY))
            } else {
                Vec::new()
            };
            let mut stroke_verts = if stroke_width > 0.0 {
                self.vert_pool
                    .pop()
                    .unwrap_or_else(|| Vec::with_capacity(INIT_VERTS_CAPACITY))
            } else {
                Vec::new()
            };

            tess.tessellate(key, &mut stroke_verts, &mut fill_verts);

            *mesh_id = UncachedMeshID(self.meshes.insert(MeshCacheEntry {
                fill_vert_range: None,
                stroke_vert_range: None,
                stroke_verts,
                fill_verts,
                stroke_width,
                antialiased: key.antialias,
            }));
        }

        UncachedMeshID(mesh_id.0)
    }

    pub fn insert_raw_mesh(
        &mut self,
        fill_verts: Vec<Vertex>,
        stroke_verts: Vec<Vertex>,
        stroke_width: f32,
        antialiased: bool,
    ) -> RawMeshID {
        RawMeshID(self.meshes.insert(MeshCacheEntry {
            fill_vert_range: None,
            stroke_vert_range: None,
            fill_verts,
            stroke_verts,
            stroke_width,
            antialiased,
        }))
    }

    pub fn raw_mesh_mut<F: FnOnce(&mut Vec<Vertex>, &mut Vec<Vertex>) -> (f32, bool)>(
        &mut self,
        mesh_id: &mut RawMeshID,
        f: F,
    ) {
        if let Some(entry) = &mut self.meshes.get_mut(mesh_id.0) {
            let (stroke_width, antialiased) = (f)(&mut entry.fill_verts, &mut entry.stroke_verts);

            entry.fill_vert_range = None;
            entry.stroke_vert_range = None;
            entry.stroke_width = stroke_width;
            entry.antialiased = antialiased;
        } else {
            let mut fill_verts = self
                .vert_pool
                .pop()
                .unwrap_or_else(|| Vec::with_capacity(INIT_VERTS_CAPACITY));
            let mut stroke_verts = Vec::new();

            let (stroke_width, antialiased) = (f)(&mut fill_verts, &mut stroke_verts);

            *mesh_id = RawMeshID(self.meshes.insert(MeshCacheEntry {
                stroke_verts,
                fill_verts,
                stroke_width,
                antialiased,
                fill_vert_range: None,
                stroke_vert_range: None,
            }));
        }
    }

    pub fn get(&self, mesh_id: MeshID) -> Option<&MeshCacheEntry> {
        self.meshes.get(mesh_id.0)
    }

    pub fn get_mut(&mut self, mesh_id: MeshID) -> Option<&mut MeshCacheEntry> {
        self.meshes.get_mut(mesh_id.0)
    }

    pub fn contains(&self, mesh_id: MeshID) -> bool {
        self.meshes.contains(mesh_id.0)
    }
}
