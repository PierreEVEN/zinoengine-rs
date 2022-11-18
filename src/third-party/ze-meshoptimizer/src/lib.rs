use ze_meshoptimizer_sys::*;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Meshlet {
    // We match meshopt_Meshlet layout
    pub vertex_offset: u32,
    pub triangle_offset: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
}
pub fn build_meshlets_bound(
    index_count: usize,
    max_vertices: usize,
    max_triangles: usize,
) -> usize {
    unsafe { meshopt_buildMeshletsBound(index_count, max_vertices, max_triangles) }
}

/// Build meshlets given vertices & indices.
#[doc = " Meshlet builder"]
#[doc = " Splits the mesh into a set of meshlets where each meshlet has a micro index buffer indexing into meshlet vertices that refer to the original vertex buffer"]
#[doc = " The resulting data can be used to render meshes using NVidia programmable mesh shading pipeline, or in other cluster-based renderers."]
#[doc = " When using buildMeshlets, vertex positions need to be provided to minimize the size of the resulting clusters."]
#[doc = " When using buildMeshletsScan, for maximum efficiency the index buffer being converted has to be optimized for vertex cache first."]
#[doc = ""]
#[doc = " meshlets must contain enough space for all meshlets, worst case size can be computed with meshopt_buildMeshletsBound"]
#[doc = " meshlet_vertices must contain enough space for all meshlets, worst case size is equal to max_meshlets * max_vertices"]
#[doc = " meshlet_triangles must contain enough space for all meshlets, worst case size is equal to max_meshlets * max_triangles * 3"]
#[doc = " vertex_positions should have float3 position in the first 12 bytes of each vertex"]
#[doc = " max_vertices and max_triangles must not exceed implementation limits (max_vertices <= 255 - not 256!, max_triangles <= 512)"]
#[doc = " cone_weight should be set to 0 when cone culling is not used, and a value between 0 and 1 otherwise to balance between cluster size and cone culling efficiency"]
pub fn build_meshlets(
    vertices_positions: &[f32],
    vertices_positions_stride: usize,
    indices: &[u32],
    max_vertices: usize,
    max_triangles: usize,
    cone_weight: f32,
) -> (Vec<Meshlet>, Vec<u32>, Vec<u8>) {
    assert!(vertices_positions_stride >= 12);

    let max_meshlets = build_meshlets_bound(indices.len(), max_vertices, max_triangles);
    let mut meshlets = vec![Meshlet::default(); max_meshlets];
    let mut meshlet_vertices = vec![0; max_meshlets * max_vertices];
    let mut meshlet_triangles = vec![0; max_meshlets * max_triangles * 3];

    let meshlet_count = unsafe {
        meshopt_buildMeshlets(
            meshlets.as_mut_ptr() as *mut _,
            meshlet_vertices.as_mut_ptr(),
            meshlet_triangles.as_mut_ptr(),
            indices.as_ptr(),
            indices.len(),
            vertices_positions.as_ptr(),
            vertices_positions.len(),
            vertices_positions_stride,
            max_vertices,
            max_triangles,
            cone_weight,
        )
    };

    meshlets.drain(meshlet_count..);
    (meshlets, meshlet_vertices, meshlet_triangles)
}
