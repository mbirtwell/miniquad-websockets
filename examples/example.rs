use example_interface::{ClientState, ConnectionId, MousePos};
use miniquad::{
    conf, info, Bindings, Buffer, BufferLayout, BufferType, Context, CustomEventPostBox,
    EventHandler, Pipeline, Shader, UserData, VertexAttribute, VertexFormat,
};
use miniquad_websockets::{WebSocketContext, WebSocketEvent, WebSocketEventKind, WebSocketSink};
use nanoserde::{DeJson, SerJson};
use std::collections::HashMap;

mod example_interface;

#[repr(C)]
struct Vec2 {
    x: f32,
    y: f32,
}

#[repr(C)]
struct Vertex {
    pos: Vec2,
}

struct Stage {
    state: HashMap<ConnectionId, ClientState>,
    pipeline: Pipeline,
    bindings: Bindings,
    #[allow(dead_code)]
    websocket_ctx: WebSocketContext<WebSocketEvent<()>>,
    websocket_sink: Option<WebSocketSink>,
}

impl Stage {
    pub fn new(ctx: &mut Context, post_box: CustomEventPostBox<WebSocketEvent<()>>) -> Stage {
        #[rustfmt::skip]
        let vertices: [Vertex; 3] = [
            Vertex {  pos: Vec2 { x: 0., y: 0. }},
            Vertex {  pos: Vec2 { x:  0.1, y: -0.2 }},
            Vertex {  pos: Vec2 { x:  0.2, y:  -0.1 }},
        ];
        let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);

        let indices: [u16; 3] = [0, 1, 2];
        let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![],
        };

        let shader = Shader::new(ctx, shader::VERTEX, shader::FRAGMENT, shader::META).unwrap();

        let pipeline = Pipeline::new(
            ctx,
            &[BufferLayout::default()],
            &[VertexAttribute::new("pos", VertexFormat::Float2)],
            shader,
        );

        info!("Starting connection");
        let mut websocket_ctx = miniquad_websockets::init(post_box).unwrap();
        websocket_ctx
            .start_connect((), "ws://127.0.0.1:8080/")
            .unwrap();

        Stage {
            state: HashMap::new(),
            pipeline,
            bindings,
            websocket_ctx,
            websocket_sink: None,
        }
    }
}

impl EventHandler<WebSocketEvent<()>> for Stage {
    fn update(&mut self, _ctx: &mut Context) {}

    fn draw(&mut self, ctx: &mut Context) {
        ctx.begin_default_pass(Default::default());

        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        for state in self.state.values() {
            ctx.apply_uniforms(&shader::Uniforms {
                offset: (state.pos.x, state.pos.y),
                color: (state.color.r, state.color.g, state.color.b),
            });
            ctx.draw(0, 3, 1);
        }
        ctx.end_render_pass();

        ctx.commit_frame();
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, x: f32, y: f32) {
        if let Some(sink) = &mut self.websocket_sink {
            let screen_size = ctx.screen_size();
            let pos = MousePos {
                x: 2. * x / screen_size.0 - 1.,
                y: -2. * y / screen_size.1 + 1.,
            };
            sink.send(pos.serialize_json()).unwrap();
        }
    }

    fn custom_event(&mut self, _ctx: &mut Context, event_data: Box<WebSocketEvent<()>>) {
        match event_data.kind {
            WebSocketEventKind::Connected(mut sink) => {
                info!("Connected");
                let pos = MousePos { x: -0.5, y: 0.5 };
                sink.send(pos.serialize_json()).unwrap();
                self.websocket_sink = Some(sink)
            }
            WebSocketEventKind::Message(msg) => {
                let state = ClientState::deserialize_json(&msg).unwrap();
                self.state.insert(state.id, state);
            }
            _ => panic!("Unhandle websocket event kind: {:?}", event_data.kind),
        }
    }
}

fn main() {
    miniquad::start_with_custom_events(conf::Conf::default(), |mut ctx, post_box| {
        UserData::owning(Stage::new(&mut ctx, post_box), ctx)
    });
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;

    uniform vec2 offset;
    uniform vec3 color;

    varying vec3 fragColor;

    void main() {
        gl_Position = vec4(pos + offset, 0, 1);
        fragColor = color;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying vec3 fragColor;

    void main() {
        gl_FragColor = vec4(fragColor, 1.0);
    }"#;

    pub const META: ShaderMeta = ShaderMeta {
        images: &[],
        uniforms: UniformBlockLayout {
            uniforms: &[
                UniformDesc::new("offset", UniformType::Float2),
                UniformDesc::new("color", UniformType::Float3),
            ],
        },
    };

    #[repr(C)]
    pub struct Uniforms {
        pub offset: (f32, f32),
        pub color: (f32, f32, f32),
    }
}
