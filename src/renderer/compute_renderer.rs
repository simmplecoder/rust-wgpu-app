use eframe::wgpu::{
    self, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry, BindingResource,
    CommandEncoderDescriptor, ComputePassDescriptor, ComputePipelineDescriptor,
    PipelineLayoutDescriptor, ShaderModuleDescriptor, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
};

use crate::{image_io::LoadedImage, renderer::ComputeRendererError};

pub struct ComputeRenderer {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ComputeRenderer {
    pub fn new(device: &wgpu::Device) -> Result<Self, ComputeRendererError> {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("to_grayscale_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/to_grayscale.wgsl").into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("to_grayscale_bind_group_layout"),
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
            label: Some("to_grayscale_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("to_grayscale_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Ok(ComputeRenderer {
            pipeline,
            bind_group_layout,
        })
    }

    fn process_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input: &wgpu::Texture,
        size: wgpu::Extent3d,
    ) -> Result<wgpu::Texture, ComputeRendererError> {
        let destination_texture = device.create_texture(&TextureDescriptor {
            label: Some("compute_renderer.destination_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let source_view = input.create_view(&TextureViewDescriptor::default());
        let destination_view = destination_texture.create_view(&TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("compute_renderer.bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&source_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&destination_view),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("compute_renderer.encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("compute_renderer.compute_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(div_ceil(size.width, 8), div_ceil(size.height, 8), 1);
        }

        queue.submit([encoder.finish()]);
        Ok(destination_texture)
    }

    pub fn process_image(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input: &LoadedImage,
    ) -> Result<LoadedImage, ComputeRendererError> {
        let size = wgpu::Extent3d {
            width: input.width,
            height: input.height,
            depth_or_array_layers: 1,
        };

        let source_texture = device.create_texture(&TextureDescriptor {
            label: Some("compute_renderer.source_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &source_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &input.rgba8,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(input.width * 4),
                rows_per_image: Some(input.height),
            },
            size,
        );

        let output_texture = self.process_texture(device, queue, &source_texture, size)?;

        let padded_bytes_per_row = padded_bytes_per_row(input.width);
        let readback_buffer_size = padded_bytes_per_row as u64 * input.height as u64;
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute_renderer.readback_buffer"),
            size: readback_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("compute_renderer.readback_encoder"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(input.height),
                },
            },
            size,
        );
        queue.submit([encoder.finish()]);

        let buffer_slice = readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(ComputeRendererError::DevicePoll)?;

        rx.recv()?.map_err(|error| {
            ComputeRendererError::BufferMap(format!("failed to map output buffer: {error}"))
        })?;

        let mapped = buffer_slice.get_mapped_range();
        let unpadded_bytes_per_row = input.width as usize * 4;
        let mut rgba8 = vec![0_u8; input.width as usize * input.height as usize * 4];
        for (row_index, source_row) in mapped
            .chunks_exact(padded_bytes_per_row as usize)
            .take(input.height as usize)
            .enumerate()
        {
            let destination_start = row_index * unpadded_bytes_per_row;
            let destination_end = destination_start + unpadded_bytes_per_row;
            rgba8[destination_start..destination_end]
                .copy_from_slice(&source_row[..unpadded_bytes_per_row]);
        }
        drop(mapped);
        readback_buffer.unmap();

        Ok(LoadedImage {
            width: input.width,
            height: input.height,
            rgba8,
        })
    }
}

fn div_ceil(value: u32, divisor: u32) -> u32 {
    value.div_ceil(divisor)
}

fn padded_bytes_per_row(width: u32) -> u32 {
    let unpadded = width * 4;
    let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    unpadded.div_ceil(alignment) * alignment
}
