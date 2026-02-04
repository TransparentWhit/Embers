use crate::reg::{RegBoxedMut, RegistryError, RegistryInitExt};
use crate::utils::{Keyed, NamespacedKey};
use anyhow::Error;
use bevy::prelude::*;
use serde::Deserialize;
use toml::Table;

#[derive(Debug)]
pub struct Block(NamespacedKey);

impl Keyed for Block {
    fn key(&self) -> &NamespacedKey {
        &self.0
    }
}

#[derive(Debug)]
pub struct BlockCollider(pub [u64; 8]);

impl BlockCollider {
    pub const RESOLUTION: i32 = 8;
    pub const VOXEL_SIZE: Vec3 = Vec3::splat(1. / Self::RESOLUTION as f32);
    pub fn coordinates(&self) -> impl Iterator<Item = IVec3> {
        self.0.iter().enumerate().flat_map(|(idx, yz)| {
            let x = idx as i32;
            let mut coords = Vec::new();
            for y in 0..8 {
                for z in 0..8 {
                    if ((yz >> (y * 8 + z)) & 1) != 0 {
                        coords.push(IVec3::new(x, y, z));
                    }
                }
            }
            coords.into_iter()
        })
    }
}

pub trait BlockColliderTemplate: Send + Sync {
    fn create(&self, config: Table) -> Result<BlockCollider, Error>;
}

impl<T: (Fn(Table) -> Result<BlockCollider, Error>) + Send + Sync> BlockColliderTemplate for T {
    fn create(&self, config: Table) -> Result<BlockCollider, Error> {
        self(config)
    }
}

#[derive(Debug)]
pub struct BlockModel {
    voxels: BlockVoxelModel,
    extra: Vec<Mesh>,
}

impl BlockModel {
    pub fn new(voxels: BlockVoxelModel) -> Self {
        Self {
            voxels,
            extra: vec![],
        }
    }
    #[inline]
    pub fn voxels(&self) -> &BlockVoxelModel {
        &self.voxels
    }
}

#[derive(Debug)]
pub struct BlockVoxelModel {
    pub voxels: u8,
}

impl BlockVoxelModel {
    pub const RESOLUTION: i32 = 2;
    pub const VOXELS: i32 = Self::RESOLUTION.pow(3);
}

pub trait BlockVoxelModelTemplate: Send + Sync {
    fn create(&self, key: NamespacedKey, config: Table) -> Result<BlockVoxelModel, Error>;
}

impl<T: (Fn(NamespacedKey, Table) -> Result<BlockVoxelModel, Error>) + Send + Sync>
    BlockVoxelModelTemplate for T
{
    fn create(&self, key: NamespacedKey, config: Table) -> Result<BlockVoxelModel, Error> {
        self(key, config)
    }
}

pub(super) fn plugin(app: &mut App) {
    app
        .init_registry::<BlockCollider>()
        .init_registry::<BlockModel>()
        .init_registry_boxed::<dyn BlockColliderTemplate>()
        .init_registry_boxed::<dyn BlockVoxelModelTemplate>()
        .add_systems(
            PreStartup,
            (|mut block_collider_templates: RegBoxedMut<dyn BlockColliderTemplate>| {
                (|| {
                    fn layered_collider(mut layers: u8) -> BlockCollider {
                        let mut yz = 0;
                        for _y in 0..8 {
                            if layers & 1 != 0 {
                                yz += 0xff;
                            }
                            layers >>= 1;
                            yz <<= 8;
                        }
                        BlockCollider([yz; 8])
                    }
                    block_collider_templates.register(
                        NamespacedKey::new_embers("empty"),
                        Box::new(|_config| Ok(BlockCollider([0x0000000000000000; 8]))),
                    )?;
                    block_collider_templates.register(
                        NamespacedKey::new_embers("full"),
                        Box::new(|_config| Ok(BlockCollider([0xffffffffffffffff; 8]))),
                    )?;
                    block_collider_templates.register(
                        NamespacedKey::new_embers("slab"),
                        Box::new(|config| {
                            #[derive(Deserialize)]
                            struct Slab {
                                variant: SlabVariant,
                            }
                            #[derive(Deserialize)]
                            #[serde(rename_all = "snake_case")]
                            enum SlabVariant {
                                Top,
                                Bottom,
                            }
                            Ok(layered_collider(match Slab::deserialize(config)?.variant {
                                SlabVariant::Top => 0x0f,
                                SlabVariant::Bottom => 0xf0,
                            }))
                        }),
                    )?;
                    block_collider_templates.register(
                        NamespacedKey::new_embers("layered"),
                        Box::new(|config| {
                            #[derive(Deserialize)]
                            struct Layered {
                                layers: u8,
                            }
                            Ok(layered_collider(Layered::deserialize(config)?.layers))
                        }),
                    )?;
                    block_collider_templates.register(
                        NamespacedKey::new_embers("custom"),
                        Box::new(|config| {
                            #[derive(Deserialize)]
                            struct Custom {
                                data: [u64; 8],
                            }
                            Ok(BlockCollider(Custom::deserialize(config)?.data))
                        }),
                    )?;
                    Ok::<(), RegistryError>(())
                })()
                .expect("Failed to register block collider templates")
            },
             |asset_server: Res<AssetServer>, mut block_voxel_model_templates: RegBoxedMut<dyn BlockVoxelModelTemplate>| {
                (|| {
                    block_voxel_model_templates.register(
                        NamespacedKey::new_embers("empty"),
                        Box::new(|_key, _config| Ok(BlockVoxelModel { voxels: 0b00000000 })),
                    )?;
                    block_voxel_model_templates.register(
                        NamespacedKey::new_embers("cube"),
                        Box::new(|_key, config| {
                            #[derive(Deserialize)]
                            struct Cube {
                                textures: Textures,
                            }
                            #[derive(Deserialize)]
                            struct Textures {
                                down: NamespacedKey,
                                up: NamespacedKey,
                                north: NamespacedKey,
                                south: NamespacedKey,
                                west: NamespacedKey,
                                east: NamespacedKey,
                            }
                            let model = Cube::deserialize(config)?;
                            Ok(BlockVoxelModel { voxels: 0b11111111 })
                        }),
                    )?;
                    block_voxel_model_templates.register(
                        NamespacedKey::new_embers("cube_all"),
                        Box::new(|_key, config| {
                            #[derive(Deserialize)]
                            struct CubeAll {
                                textures: Textures,
                            }
                            #[derive(Deserialize)]
                            struct Textures {
                                all: NamespacedKey,
                            }
                            let model = CubeAll::deserialize(config)?;
                            Ok(BlockVoxelModel { voxels: 0b11111111 })
                        }),
                    )?;
                    block_voxel_model_templates.register(
                        NamespacedKey::new_embers("bottom_slab"),
                        Box::new(|_key, config| {
                            #[derive(Deserialize)]
                            struct BottomSlab {
                                textures: Textures,
                            }
                            #[derive(Deserialize)]
                            struct Textures {
                                side: NamespacedKey,
                                top: NamespacedKey,
                                bottom: NamespacedKey,
                            }
                            let model = BottomSlab::deserialize(config)?;
                            Ok(BlockVoxelModel { voxels: 0b11001100 })
                        }),
                    )?;
                    Ok::<(), RegistryError>(())
                })()
                .expect("Failed to register block model voxel templates")
            },
            ));
}
