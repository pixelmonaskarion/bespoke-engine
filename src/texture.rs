use image::GenericImageView;
use anyhow::*;
use wgpu::{BindGroup, BindGroupLayout};

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: Option<wgpu::Sampler>,
    pub layout: BindGroupLayout,
    pub binding: BindGroup,
}

impl Texture {
    #[allow(dead_code)]
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8], 
        label: &str,
        filter_mode: Option<wgpu::FilterMode>,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label), None, None, filter_mode)
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
        format: Option<wgpu::TextureFormat>,
        sample_type: Option<wgpu::TextureSampleType>,
        filter_mode: Option<wgpu::FilterMode>,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: format.unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb),
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[format.unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb)],
            }
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: filter_mode.unwrap_or(wgpu::FilterMode::Nearest),
                min_filter: filter_mode.unwrap_or(wgpu::FilterMode::Nearest),
                mipmap_filter: filter_mode.unwrap_or(wgpu::FilterMode::Nearest),
                ..Default::default()
            }
        );
        let layout = Self::layout(device, sample_type, label);
        let binding = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    }
                ],
                label: Some(&(label.unwrap_or("").to_owned() + " Binding")),
            }
        );
        
        Ok(Self { texture: texture, view, sampler: Some(sampler), layout, binding })
    }

    pub fn blank_texture(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Temp Draw Texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[format],
            }
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        //create a uniform binding for that texture
        let layout = Texture::layout(device, None, None);
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );
        let binding = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    }
                ],
                label: None,
            }
        );
        Self {
            binding,
            layout,
            sampler: Some(sampler),
            texture,
            view,
        }
    }

    pub fn layout(device: &wgpu::Device, sample_type: Option<wgpu::TextureSampleType>, label: Option<&str>) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: sample_type.unwrap_or(wgpu::TextureSampleType::Float { filterable: true }),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some(&(label.unwrap_or("").to_owned() + " Layout")),
        })
    }

    pub fn normalized_dimensions(&self) -> (f32, f32) {
        let dist = ((self.texture.width() as f32).powf(2.0)+(self.texture.height() as f32).powf(2.0)).sqrt();
        (self.texture.width() as f32/dist, self.texture.height() as f32/dist)
    }

    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub fn depth_layout(device: &wgpu::Device, sample_type: Option<wgpu::TextureSampleType>, label: Option<&str>) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: sample_type.unwrap_or(wgpu::TextureSampleType::Depth),
                    },
                    count: None,
                },
                // wgpu::BindGroupLayoutEntry {
                //     binding: 1,
                //     visibility: wgpu::ShaderStages::FRAGMENT,
                //     // This should match the filterable field of the
                //     // corresponding Texture entry above.
                //     ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                //     count: None,
                // },
            ],
            label: Some(&(label.unwrap_or("").to_owned() + " Layout")),
        })
    }
    
    pub fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, label: &str) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            view_formats: &[Self::DEPTH_FORMAT],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // let sampler = device.create_sampler(
        //     &wgpu::SamplerDescriptor {
        //         address_mode_u: wgpu::AddressMode::ClampToEdge,
        //         address_mode_v: wgpu::AddressMode::ClampToEdge,
        //         address_mode_w: wgpu::AddressMode::ClampToEdge,
        //         mag_filter: wgpu::FilterMode::Nearest,
        //         min_filter: wgpu::FilterMode::Nearest,
        //         mipmap_filter: wgpu::FilterMode::Nearest,
        //         ..Default::default()
        //     }
        // );
        let layout = Texture::depth_layout(device, None, None);
        let binding = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    // wgpu::BindGroupEntry {
                    //     binding: 1,
                    //     resource: wgpu::BindingResource::Sampler(&sampler),
                    // }
                ],
                label: None,
            }
        );
        Self { 
            texture, 
            view,
            sampler: None,
            layout,
            binding,
        }
    }
}