use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderContext},
};

use crate::WORKGROUP_SIZE;

pub fn compute_pipeline_descriptor(
    shader: Handle<Shader>,
    entry_point: &str,
    bind_group_layout: &BindGroupLayout,
    shader_defs: Vec<ShaderDefVal>,
) -> ComputePipelineDescriptor {
    ComputePipelineDescriptor {
        label: None,
        layout: vec![bind_group_layout.clone()],
        shader,
        shader_defs,
        entry_point: Cow::from(entry_point.to_owned()),
        push_constant_ranges: vec![],
    }
}

fn compute_pass<'a>(
    render_context: &'a mut RenderContext,
    bind_group: &'a BindGroup,
    pipeline_cache: &'a PipelineCache,
    pipeline: CachedComputePipelineId,
) -> wgpu::ComputePass<'a> {
    let mut pass = render_context
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor::default());

    pass.set_bind_group(0, bind_group, &[]);

    let pipeline = pipeline_cache.get_compute_pipeline(pipeline).unwrap();
    pass.set_pipeline(pipeline);
    pass
}

pub fn run_compute_pass(
    render_context: &mut RenderContext,
    bind_group: &BindGroup,
    pipeline_cache: &PipelineCache,
    pipeline: CachedComputePipelineId,
    particle_count: u32,
) {
    let mut pass = compute_pass(render_context, bind_group, pipeline_cache, pipeline);
    pass.dispatch_workgroups(particle_count / WORKGROUP_SIZE, 1, 1);
}

pub fn run_compute_pass_2d(
    render_context: &mut RenderContext,
    bind_group: &BindGroup,
    pipeline_cache: &PipelineCache,
    pipeline: CachedComputePipelineId,
    width: u32,
    height: u32,
) {
    let mut pass = compute_pass(render_context, bind_group, pipeline_cache, pipeline);

    pass.dispatch_workgroups(width / WORKGROUP_SIZE, height / WORKGROUP_SIZE, 1);
}
