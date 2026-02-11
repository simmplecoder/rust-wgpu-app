use crate::image_io::LoadedImage;

pub fn run(
    device: &eframe::wgpu::Device,
    queue: &eframe::wgpu::Queue,
    input: &LoadedImage,
) -> Result<LoadedImage, String> {
    let texture_extent = eframe::wgpu::Extent3d {
        width: input.width,
        height: input.height,
        depth_or_array_layers: 1,
    };

    let source_texture = device.create_texture(&eframe::wgpu::TextureDescriptor {
        label: Some("remove_red.source_texture"),
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: eframe::wgpu::TextureDimension::D2,
        format: eframe::wgpu::TextureFormat::Rgba8Unorm,
        usage: eframe::wgpu::TextureUsages::TEXTURE_BINDING | eframe::wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        eframe::wgpu::TexelCopyTextureInfo {
            texture: &source_texture,
            mip_level: 0,
            origin: eframe::wgpu::Origin3d::ZERO,
            aspect: eframe::wgpu::TextureAspect::All,
        },
        &input.rgba8,
        eframe::wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(input.width * 4),
            rows_per_image: Some(input.height),
        },
        texture_extent,
    );

    let destination_texture = device.create_texture(&eframe::wgpu::TextureDescriptor {
        label: Some("remove_red.destination_texture"),
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: eframe::wgpu::TextureDimension::D2,
        format: eframe::wgpu::TextureFormat::Rgba8Unorm,
        usage: eframe::wgpu::TextureUsages::STORAGE_BINDING | eframe::wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let source_view = source_texture.create_view(&eframe::wgpu::TextureViewDescriptor::default());
    let destination_view =
        destination_texture.create_view(&eframe::wgpu::TextureViewDescriptor::default());

    let bind_group_layout =
        device.create_bind_group_layout(&eframe::wgpu::BindGroupLayoutDescriptor {
            label: Some("remove_red.bind_group_layout"),
            entries: &[
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: eframe::wgpu::ShaderStages::COMPUTE,
                    ty: eframe::wgpu::BindingType::Texture {
                        sample_type: eframe::wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: eframe::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                eframe::wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: eframe::wgpu::ShaderStages::COMPUTE,
                    ty: eframe::wgpu::BindingType::StorageTexture {
                        access: eframe::wgpu::StorageTextureAccess::WriteOnly,
                        format: eframe::wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: eframe::wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

    let bind_group = device.create_bind_group(&eframe::wgpu::BindGroupDescriptor {
        label: Some("remove_red.bind_group"),
        layout: &bind_group_layout,
        entries: &[
            eframe::wgpu::BindGroupEntry {
                binding: 0,
                resource: eframe::wgpu::BindingResource::TextureView(&source_view),
            },
            eframe::wgpu::BindGroupEntry {
                binding: 1,
                resource: eframe::wgpu::BindingResource::TextureView(&destination_view),
            },
        ],
    });

    let shader = device.create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
        label: Some("remove_red.shader"),
        source: eframe::wgpu::ShaderSource::Wgsl(include_str!("../shaders/remove_red.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&eframe::wgpu::PipelineLayoutDescriptor {
        label: Some("remove_red.pipeline_layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline =
        device.create_compute_pipeline(&eframe::wgpu::ComputePipelineDescriptor {
            label: Some("remove_red.pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: eframe::wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

    let padded_bytes_per_row = padded_bytes_per_row(input.width);
    let readback_buffer_size = padded_bytes_per_row as u64 * input.height as u64;
    let readback_buffer = device.create_buffer(&eframe::wgpu::BufferDescriptor {
        label: Some("remove_red.readback_buffer"),
        size: readback_buffer_size,
        usage: eframe::wgpu::BufferUsages::COPY_DST | eframe::wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&eframe::wgpu::CommandEncoderDescriptor {
        label: Some("remove_red.encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&eframe::wgpu::ComputePassDescriptor {
            label: Some("remove_red.compute_pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups(div_ceil(input.width, 8), div_ceil(input.height, 8), 1);
    }

    encoder.copy_texture_to_buffer(
        eframe::wgpu::TexelCopyTextureInfo {
            texture: &destination_texture,
            mip_level: 0,
            origin: eframe::wgpu::Origin3d::ZERO,
            aspect: eframe::wgpu::TextureAspect::All,
        },
        eframe::wgpu::TexelCopyBufferInfo {
            buffer: &readback_buffer,
            layout: eframe::wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(input.height),
            },
        },
        texture_extent,
    );

    queue.submit([encoder.finish()]);

    let buffer_slice = readback_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(eframe::wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = device
        .poll(eframe::wgpu::PollType::wait_indefinitely())
        .map_err(|e| format!("device poll failed: {e}"))?;

    rx.recv()
        .map_err(|error| format!("failed waiting for mapped buffer: {error}"))?
        .map_err(|error| format!("failed to map output buffer: {error}"))?;

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

fn div_ceil(value: u32, divisor: u32) -> u32 {
    value.div_ceil(divisor)
}

fn padded_bytes_per_row(width: u32) -> u32 {
    let unpadded = width * 4;
    let alignment = eframe::wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    unpadded.div_ceil(alignment) * alignment
}
