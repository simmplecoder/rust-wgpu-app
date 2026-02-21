use eframe::wgpu::{
    self, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, CommandEncoder, ComputePassDescriptor, ComputePipeline,
    ComputePipelineDescriptor, Device, PipelineCompilationOptions, PipelineLayoutDescriptor,
    ShaderModuleDescriptor, TextureView,
};

pub struct GrayscaleEncodeArgs<'a> {
    pub device: &'a Device,
    pub encoder: &'a mut CommandEncoder,
    pub src_view: &'a TextureView,
    pub dst_view: &'a TextureView,
    pub width: u32,
    pub height: u32,
}

pub struct GrayscalePass {
    pub pipeline: ComputePipeline,
    pub bind_group_layout: BindGroupLayout,
}

impl GrayscalePass {
    pub fn new(device: &wgpu::Device) -> GrayscalePass {
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Grayscale.shader_module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/to_grayscale.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Grayscale.bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("grayscale.pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("grayscale.pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });

        GrayscalePass {
            pipeline: pipeline,
            bind_group_layout,
        }
    }

    pub fn encode(&self, args: GrayscaleEncodeArgs<'_>) {
        let bind_group = args.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Grayscale.bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&args.src_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&args.dst_view),
                },
            ],
        });

        {
            let mut compute_pass = args.encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("compute_renderer.compute_pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(args.width.div_ceil(8), args.height.div_ceil(8), 1);
        }
    }
}
