use bevy::{
    prelude::*,
    render::{
        render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
        render_phase::TrackedRenderPass,
        render_resource::{
            CachedPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
            DepthStencilState, Face, FragmentState, FrontFace, LoadOp, MultisampleState,
            Operations, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
            RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipelineCache,
            RenderPipelineDescriptor, StencilFaceState, StencilOperation, StencilState,
            TextureFormat, VertexState,
        },
        renderer::RenderContext,
    },
};

use crate::{OutlineResources, JFA_INIT_SHADER_HANDLE};

pub const TEXTURE_FORMAT: TextureFormat = TextureFormat::Rg16Snorm;

pub struct CoordsPipeline {
    cached: CachedPipelineId,
}

impl FromWorld for CoordsPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut pipeline_cache = world.get_resource_mut::<RenderPipelineCache>().unwrap();
        let cached = pipeline_cache.queue(RenderPipelineDescriptor {
            label: Some("outline_coords_pipeline".into()),
            layout: None,
            vertex: VertexState {
                shader: JFA_INIT_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState {
                        compare: CompareFunction::Equal,
                        fail_op: StencilOperation::Keep,
                        depth_fail_op: StencilOperation::Keep,
                        pass_op: StencilOperation::Keep,
                    },
                    back: StencilFaceState::IGNORE,
                    read_mask: !0,
                    write_mask: 0,
                },
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: JFA_INIT_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TEXTURE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }],
            }),
        });

        CoordsPipeline { cached }
    }
}

pub struct CoordsNode;

impl CoordsNode {
    pub const IN_STENCIL: &'static str = "in_stencil";
    pub const OUT_COORDS: &'static str = "out_coords";
}

impl Node for CoordsNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_STENCIL, SlotType::TextureView)]
    }

    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::OUT_COORDS, SlotType::TextureView)]
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let res = world.get_resource::<OutlineResources>().unwrap();
        graph
            .set_output(
                Self::OUT_COORDS,
                res.jfa_primary_output.default_view.clone(),
            )
            .unwrap();

        let stencil = graph.get_input_texture(Self::IN_STENCIL).unwrap();

        let pipeline = world.get_resource::<CoordsPipeline>().unwrap();
        let pipeline_cache = world.get_resource::<RenderPipelineCache>().unwrap();
        let cached_pipeline = match pipeline_cache.get(pipeline.cached) {
            Some(c) => c,
            // Still queued.
            None => {
                return Ok(());
            }
        };

        let render_pass = render_context
            .command_encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("outline_coords"),
                color_attachments: &[RenderPassColorAttachment {
                    view: &res.jfa_primary_output.default_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(
                            Color::RgbaLinear {
                                red: -1.0,
                                green: -1.0,
                                blue: 0.0,
                                alpha: 0.0,
                            }
                            .into(),
                        ),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: stencil,
                    depth_ops: None,
                    stencil_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: false,
                    }),
                }),
            });
        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        tracked_pass.set_render_pipeline(&cached_pipeline);
        tracked_pass.set_stencil_reference(!0);
        tracked_pass.draw(0..3, 0..1);

        Ok(())
    }
}
