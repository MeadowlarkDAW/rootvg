use ahash::AHashMap;
use std::{hash::Hash, ops::Range};
use thunderdome::Arena;

use crate::Vertex;

use super::{MeshBuilder, MeshBuilderInner, Tessellator};

const INIT_VERTS_SIZE: usize = 128;
const INIT_POOL_CAPACITY: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshID(thunderdome::Index);

struct MeshCacheBuilderEntry {
    used: bool,
    mesh_id: MeshID,
}

pub(crate) struct MeshCacheEntry {
    pub stroke_verts: Vec<Vertex>,
    pub fill_verts: Vec<Vertex>,
    pub stroke_width: f32,
    pub antialiased: bool,
    pub fill_vert_range: Option<Range<usize>>,
    pub stroke_vert_range: Option<Range<usize>>,
}

// TODO: An option to use a concurrent cache
pub(crate) struct MeshCache {
    builder_to_id_map: AHashMap<MeshBuilderInner, MeshCacheBuilderEntry>,
    meshes: thunderdome::Arena<MeshCacheEntry>,
}

impl MeshCache {
    pub fn new() -> Self {
        Self {
            builder_to_id_map: AHashMap::with_capacity(INIT_POOL_CAPACITY),
            meshes: Arena::with_capacity(INIT_POOL_CAPACITY),
        }
    }

    pub fn begin_frame(&mut self) {
        for entry in self.builder_to_id_map.values_mut() {
            entry.used = false;
        }
        for (_, entry) in self.meshes.iter_mut() {
            entry.fill_vert_range = None;
            entry.stroke_vert_range = None;
        }
    }

    pub fn end_frame(&mut self) {
        self.builder_to_id_map.retain(|_, entry| entry.used);
        self.meshes.retain(|_, entry| {
            entry.fill_vert_range.is_some() || entry.stroke_vert_range.is_some()
        });
    }

    pub fn build_mesh_cached(
        &mut self,
        tess: &mut Tessellator,
        mesh_builder: &MeshBuilder,
        antialias: bool,
        force_rebuild: bool,
    ) -> MeshID {
        let antialias = antialias && mesh_builder.inner.antialias;

        if let Some(entry) = self.builder_to_id_map.get_mut(&mesh_builder.inner) {
            entry.used = true;

            if !self.meshes.contains(entry.mesh_id.0) || force_rebuild {
                let stroke_width = f32::from_ne_bytes(mesh_builder.inner.stroke_width_bytes);

                let mut fill_verts = if mesh_builder.inner.fill {
                    Vec::with_capacity(INIT_VERTS_SIZE)
                } else {
                    Vec::new()
                };
                let mut stroke_verts = if stroke_width > 0.0 {
                    Vec::with_capacity(INIT_VERTS_SIZE)
                } else {
                    Vec::new()
                };

                tess.tessellate(mesh_builder, &mut stroke_verts, &mut fill_verts, antialias);

                entry.mesh_id = MeshID(self.meshes.insert(MeshCacheEntry {
                    fill_vert_range: None,
                    stroke_vert_range: None,
                    stroke_verts,
                    fill_verts,
                    stroke_width,
                    antialiased: antialias,
                }));
            }

            entry.mesh_id
        } else {
            let stroke_width = f32::from_ne_bytes(mesh_builder.inner.stroke_width_bytes);

            let mut fill_verts = if mesh_builder.inner.fill {
                Vec::with_capacity(INIT_VERTS_SIZE)
            } else {
                Vec::new()
            };
            let mut stroke_verts = if stroke_width > 0.0 {
                Vec::with_capacity(INIT_VERTS_SIZE)
            } else {
                Vec::new()
            };

            tess.tessellate(mesh_builder, &mut stroke_verts, &mut fill_verts, antialias);

            let mesh_id = MeshID(self.meshes.insert(MeshCacheEntry {
                fill_vert_range: None,
                stroke_vert_range: None,
                stroke_verts,
                fill_verts,
                stroke_width,
                antialiased: antialias,
            }));

            self.builder_to_id_map.insert(
                mesh_builder.inner.clone(),
                MeshCacheBuilderEntry {
                    used: true,
                    mesh_id,
                },
            );

            mesh_id
        }
    }

    pub fn build_mesh_uncached(
        &mut self,
        tess: &mut Tessellator,
        mesh_builder: &MeshBuilder,
        antialias: bool,
        prev_mesh_id: &mut Option<MeshID>,
    ) -> MeshID {
        let antialias = antialias && mesh_builder.inner.antialias;

        if let Some(entry) = &mut self.meshes.get_mut(
            prev_mesh_id
                .map(|id| id.0)
                .unwrap_or(thunderdome::Index::DANGLING),
        ) {
            entry.stroke_width = f32::from_ne_bytes(mesh_builder.inner.stroke_width_bytes);
            entry.antialiased = antialias;

            tess.tessellate(
                mesh_builder,
                &mut entry.stroke_verts,
                &mut entry.fill_verts,
                antialias,
            );

            prev_mesh_id.unwrap()
        } else {
            let stroke_width = f32::from_ne_bytes(mesh_builder.inner.stroke_width_bytes);

            let mut fill_verts = if mesh_builder.inner.fill {
                Vec::with_capacity(INIT_VERTS_SIZE)
            } else {
                Vec::new()
            };
            let mut stroke_verts = if stroke_width > 0.0 {
                Vec::with_capacity(INIT_VERTS_SIZE)
            } else {
                Vec::new()
            };

            tess.tessellate(mesh_builder, &mut stroke_verts, &mut fill_verts, antialias);

            let mesh_id = MeshID(self.meshes.insert(MeshCacheEntry {
                fill_vert_range: None,
                stroke_vert_range: None,
                stroke_verts,
                fill_verts,
                stroke_width,
                antialiased: antialias,
            }));

            *prev_mesh_id = Some(mesh_id);

            mesh_id
        }
    }

    pub fn insert_raw_mesh(
        &mut self,
        stroke_verts: Vec<Vertex>,
        fill_verts: Vec<Vertex>,
        stroke_width: f32,
        antialiased: bool,
    ) -> MeshID {
        MeshID(self.meshes.insert(MeshCacheEntry {
            fill_vert_range: None,
            stroke_vert_range: None,
            stroke_verts,
            fill_verts,
            stroke_width,
            antialiased,
        }))
    }

    pub fn get(&self, mesh_id: MeshID) -> Option<&MeshCacheEntry> {
        self.meshes.get(mesh_id.0)
    }

    pub fn get_mut(&mut self, mesh_id: MeshID) -> Option<&mut MeshCacheEntry> {
        self.meshes.get_mut(mesh_id.0)
    }
}
