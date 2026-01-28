use crate::dim::block::{Block, BlockCollider, BlockModel, BlockVoxelModel};
use crate::reg::{Registry, ValueIndex};
use crate::utils::Keyed;
use avian3d::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

#[derive(Component)]
#[component(on_insert = chunk_insertion_hook, on_remove = chunk_removal_hook)]
pub struct Chunk {
    blocks: [Block; Self::SIZE.pow(3) as usize],
}

fn chunk_insertion_hook(mut world: DeferredWorld, context: HookContext) {
    let collider_reg = world.resource::<Registry<BlockCollider>>();
    let voxel_coords = world
        .get::<Chunk>(context.entity)
        .unwrap()
        .blocks
        .iter()
        .enumerate()
        .flat_map(|(idx, block)| {
            let mut idx = idx as i32;
            let block_voxel_z = (idx % Chunk::SIZE) * BlockCollider::RESOLUTION;
            idx /= Chunk::SIZE;
            let block_voxel_y = (idx % Chunk::SIZE) * BlockCollider::RESOLUTION;
            let block_voxel_x = (idx / Chunk::SIZE) * BlockCollider::RESOLUTION;
            collider_reg
                .get(block.key())
                .expect("Unknown block in chunk")
                .coordinates()
                .map(move |mut coord| {
                    coord.x += block_voxel_x;
                    coord.y += block_voxel_y;
                    coord.z += block_voxel_z;
                    coord
                })
        })
        .collect::<Box<[_]>>();
    let model_reg = world.resource::<Registry<BlockModel>>();
    let mesh = world
        .resource::<GreedyChunkMesher>()
        .generate_mesh(
            world
                .get::<Chunk>(context.entity)
                .unwrap()
                .blocks
                .iter()
                .flat_map(|block| {
                    let model_index = model_reg
                        .get_index(block.key())
                        .expect("Unknown block in chunk");
                    let voxels = model_reg[model_index].voxels().voxels;
                    (0..BlockVoxelModel::VOXELS).map(move |idx| BlockVoxel {
                        block_type: if ((voxels >> idx) & 1) == 0 {
                            Some(model_index)
                        } else {
                            None
                        },
                    })
                })
                .collect::<Box<[_]>>()
                .as_ref(),
        )
        .mesh();
    let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);
    world.commands().entity(context.entity).insert((
        Collider::voxels(BlockCollider::VOXEL_SIZE, voxel_coords.as_ref()),
        Mesh3d(mesh),
        MeshMaterial3d::<StandardMaterial>(todo!()),
    ));
}

fn chunk_removal_hook(mut world: DeferredWorld, context: HookContext) {
    world
        .commands()
        .entity(context.entity)
        .remove::<(Collider, Mesh3d, MeshMaterial3d<StandardMaterial>)>();
}

impl Chunk {
    pub const SIZE: i32 = 16;
    pub const VOXEL_MODEL_SIZE: i32 = Self::SIZE * BlockVoxelModel::RESOLUTION;
}

#[derive(Clone)]
pub struct ChunkMeshData {
    // TODO
}

impl ChunkMeshData {
    fn mesh(&self) -> Mesh {
        todo!();
        /*let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))*/
    }
}

pub struct BlockVoxel {
    pub block_type: Option<ValueIndex<BlockModel>>,
}

#[derive(Resource)]
pub struct GreedyChunkMesher {}

impl GreedyChunkMesher {
    fn generate_mesh(&self, voxels: &[BlockVoxel]) -> ChunkMeshData {
        todo!()
    }
}

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(GreedyChunkMesher {});
}
