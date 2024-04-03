use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
    },
};

use crate::{
    compute_utils::{compute_pipeline_descriptor, run_compute_pass},
    particle_config::ParticleConfig,
    particle_system::ParticleSystemRender,
    sort_spatial_hash_grid::sort_spatial_hash_grid,
    ParticleSystem, HEIGHT, WIDTH, WORKGROUP_SIZE,
};

#[derive(Resource, Clone)]
pub struct ParticleUpdatePipelineConfig {
    update_bind_group_layout: BindGroupLayout,
    sort_bind_group_layout: BindGroupLayout,
    update_positions_pipeline: CachedComputePipelineId,
    update_velocities_pipeline: CachedComputePipelineId,
    update_spatial_hash_grid_pipeline: CachedComputePipelineId,
    pub sort_pipeline: CachedComputePipelineId,
    pub calculate_offsets_pipeline: CachedComputePipelineId,
}

pub struct UpdateParticlesNode {
    particle_system: QueryState<Entity, With<ParticleSystem>>,
    update_state: ParticleUpdateState,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct UpdateParticlesRenderLabel;

#[derive(Default, Clone, Eq, PartialEq)]
enum ParticleUpdateState {
    #[default]
    Loading,
    UpdateSpatialHashGrid,
    SortSpatialIndices,
    CalculateOffsets,
    UpdateVelocities,
    UpdatePositions,
}

pub fn create_update_bind_group(
    render_device: &RenderDevice,
    update_pipeline: &ParticleUpdatePipelineConfig,
    particle_system_render: &ParticleSystemRender,
) -> BindGroup {
    render_device.create_bind_group(
        None,
        &update_pipeline.update_bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particle_system_render
                    .particle_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: particle_system_render
                    .particle_config_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: particle_system_render
                    .attraction_matrix_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: particle_system_render
                    .delta_time_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
            BindGroupEntry {
                binding: 4,
                resource: particle_system_render
                    .spatial_indices_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
            BindGroupEntry {
                binding: 5,
                resource: particle_system_render
                    .spatial_offsets_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
        ],
    )
}

fn create_update_bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
    render_device.create_bind_group_layout(
        "update_bind_group_layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform {},
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform {},
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 4,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 5,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    )
}

// @group(0) @binding(0) var<storage, read_write> entries: array<Entry>;
// @group(0) @binding(1) var<uniform> num_entries: u32;
// @group(0) @binding(2) var<uniform> group_width: u32;
// @group(0) @binding(3) var<uniform> group_height: u32;
// @group(0) @binding(4) var<uniform> step_index: u32;
// @group(0) @binding(5) var<storage, read_write> offsets: array<u32>;

pub fn create_sort_bind_group(
    render_device: &RenderDevice,
    update_pipeline: &ParticleUpdatePipelineConfig,
    particle_system_render: &ParticleSystemRender,
) -> BindGroup {
    render_device.create_bind_group(
        None,
        &update_pipeline.sort_bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particle_system_render
                    .spatial_indices_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: particle_system_render
                    .num_entries_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: particle_system_render
                    .group_width_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: particle_system_render
                    .group_height_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
            BindGroupEntry {
                binding: 4,
                resource: particle_system_render
                    .step_index_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
            BindGroupEntry {
                binding: 5,
                resource: particle_system_render
                    .spatial_offsets_buffer
                    .as_ref()
                    .unwrap()
                    .as_entire_binding(),
            },
        ],
    )
}

fn create_sort_bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
    render_device.create_bind_group_layout(
        "sort_bind_group_layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<u32>() as _), // is this necessary?
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<u32>() as _), // is this necessary?
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<u32>() as _), // is this necessary?
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 4,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<u32>() as _), // is this necessary?
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 5,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    )
}

impl FromWorld for ParticleUpdatePipelineConfig {
    fn from_world(world: &mut World) -> Self {
        let update_bind_group_layout =
            create_update_bind_group_layout(&world.resource::<RenderDevice>());

        let particle_update_shader = &world.resource::<AssetServer>().load("particle_update.wgsl");
        //   let gpu_sort_shader = world.resource::<AssetServer>().load("gpu_sort.wgsl");

        let shader_defs = vec![
            ShaderDefVal::UInt("WIDTH".into(), (WIDTH as u32).into()),
            ShaderDefVal::UInt("HEIGHT".into(), (HEIGHT as u32).into()),
            ShaderDefVal::UInt("WORKGROUP_SIZE".into(), WORKGROUP_SIZE),
        ];

        let pipeline_cache = &world.resource_mut::<PipelineCache>();

        let update_velocities_pipeline =
            pipeline_cache.queue_compute_pipeline(compute_pipeline_descriptor(
                particle_update_shader.clone(),
                "update_velocities",
                &update_bind_group_layout,
                shader_defs.clone(),
            ));

        let update_positions_pipeline =
            pipeline_cache.queue_compute_pipeline(compute_pipeline_descriptor(
                particle_update_shader.clone(),
                "update_positions",
                &update_bind_group_layout,
                shader_defs.clone(),
            ));

        let update_spatial_hash_grid_pipeline =
            pipeline_cache.queue_compute_pipeline(compute_pipeline_descriptor(
                particle_update_shader.clone(),
                "update_spatial_hash_grid",
                &update_bind_group_layout,
                shader_defs.clone(),
            ));

        let sort_bind_group_layout =
            create_sort_bind_group_layout(&world.resource::<RenderDevice>());

        let sort_shader = &world.resource::<AssetServer>().load("gpu_sort.wgsl");

        let pipeline_cache = &world.resource_mut::<PipelineCache>();

        let sort_pipeline = pipeline_cache.queue_compute_pipeline(compute_pipeline_descriptor(
            sort_shader.clone(),
            "Sort",
            &sort_bind_group_layout,
            vec![],
        ));

        let calculate_offsets_pipeline =
            pipeline_cache.queue_compute_pipeline(compute_pipeline_descriptor(
                sort_shader.clone(),
                "CalculateOffsets",
                &sort_bind_group_layout,
                vec![],
            ));

        ParticleUpdatePipelineConfig {
            update_bind_group_layout,
            sort_bind_group_layout,
            update_velocities_pipeline,
            update_positions_pipeline,
            update_spatial_hash_grid_pipeline,
            sort_pipeline,
            calculate_offsets_pipeline,
        }
    }
}

impl render_graph::Node for UpdateParticlesNode {
    fn update(&mut self, world: &mut World) {
        let mut systems = world.query_filtered::<Entity, With<ParticleSystem>>();
        let pipeline_config = world.resource::<ParticleUpdatePipelineConfig>();
        let pipeline_cache = world.resource::<PipelineCache>();

        if systems.get_single(world).is_ok() {
            // if the corresponding pipeline has loaded, transition to the next stage
            self.update_state(pipeline_cache, pipeline_config);
        }
        // Update the query for the run step
        self.particle_system.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let render_queue = world.resource::<RenderQueue>();
        let pipeline_config = world.resource::<ParticleUpdatePipelineConfig>();
        let particle_system_render = world.resource::<ParticleSystemRender>();
        let particle_config = world.resource::<ParticleConfig>();

        for _ in self.particle_system.iter_manual(world) {
            if let Some(pipeline) = match self.update_state {
                ParticleUpdateState::Loading => None,
                ParticleUpdateState::UpdateSpatialHashGrid => {
                    Some(pipeline_config.update_spatial_hash_grid_pipeline)
                }
                ParticleUpdateState::SortSpatialIndices => Some(pipeline_config.sort_pipeline),
                ParticleUpdateState::CalculateOffsets => {
                    Some(pipeline_config.calculate_offsets_pipeline)
                }
                ParticleUpdateState::UpdateVelocities => {
                    Some(pipeline_config.update_velocities_pipeline)
                }
                ParticleUpdateState::UpdatePositions => {
                    Some(pipeline_config.update_positions_pipeline)
                }
            } {
                if self.update_state == ParticleUpdateState::SortSpatialIndices
                    || self.update_state == ParticleUpdateState::CalculateOffsets
                {
                    sort_spatial_hash_grid(
                        particle_config.n as u32,
                        &render_queue,
                        render_context.render_device(),
                        particle_system_render,
                        pipeline_config,
                        pipeline_cache,
                    );
                } else {
                    run_compute_pass(
                        render_context,
                        &particle_system_render.update_bind_group.as_ref().unwrap(),
                        pipeline_cache,
                        pipeline,
                        particle_config.n as u32,
                    );
                }
            }
        }

        Ok(())
    }
}

impl UpdateParticlesNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            particle_system: QueryState::new(world),
            update_state: ParticleUpdateState::default(),
        }
    }

    fn update_state(
        &mut self,
        pipeline_cache: &PipelineCache,
        pipeline: &ParticleUpdatePipelineConfig,
    ) {
        match self.update_state {
            ParticleUpdateState::Loading => {
                if let CachedPipelineState::Ok(_) = pipeline_cache
                    .get_compute_pipeline_state(pipeline.update_spatial_hash_grid_pipeline)
                {
                    self.update_state = ParticleUpdateState::UpdateSpatialHashGrid;
                }
            }
            ParticleUpdateState::UpdateSpatialHashGrid => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_velocities_pipeline)
                {
                    self.update_state = ParticleUpdateState::SortSpatialIndices;
                }
            }
            ParticleUpdateState::SortSpatialIndices => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.sort_pipeline)
                {
                    self.update_state = ParticleUpdateState::CalculateOffsets;
                }
            }
            ParticleUpdateState::CalculateOffsets => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.calculate_offsets_pipeline)
                {
                    self.update_state = ParticleUpdateState::UpdateVelocities;
                }
            }
            ParticleUpdateState::UpdateVelocities => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_positions_pipeline)
                {
                    self.update_state = ParticleUpdateState::UpdatePositions;
                }
            }
            ParticleUpdateState::UpdatePositions => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_velocities_pipeline)
                {
                    self.update_state = ParticleUpdateState::UpdateSpatialHashGrid;
                }
            }
        }
    }
}
