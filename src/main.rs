extern crate amethyst;
extern crate tiled;

use amethyst::{
    assets::Processor,
    core::{frame_limiter::FrameRateLimitStrategy, transform::{TransformBundle}},
    ecs::{ReadExpect, Resources, SystemData},
    // error::Error,
    input::{InputBundle, StringBindings},
    // prelude::{Builder, World},
    renderer::{
        // camera::Camera,
        pass::DrawFlat2DDesc,
        rendy::{
            factory::Factory,
            graph::{
                render::{RenderGroupDesc, SubpassBuilder},
                GraphBuilder,
            },
            hal::{format::Format, image},
        },
        sprite::{SpriteSheet},
        types::DefaultBackend,
        GraphCreator, RenderingSystem,
    },
    utils::application_root_dir,
    window::{ScreenDimensions, Window, WindowBundle},
    Application,
    GameDataBuilder, 
};

use std::sync::Arc;
use std::time::Duration;

mod entities;
mod states;
mod components;
mod resources;
mod systems;

use systems::{
    MarineAccelerationSystem,
    MarineAnimationSystem,
    AttackSystem,
    BulletImpactAnimationSystem,
    BulletAnimationSystem,
    BulletCollisionSystem,
    CameraMotionSystem,
    MarineCollisionSystem
};

pub const SCALE: f32 = 2.;
pub const BG_Z_TRANSFORM: f32 = -30.;
pub const PLATFORM_Z_TRANSFORM: f32 = -10.;

pub const MARINE_MAX_VELOCITY: f32 = 6.0;

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let root = application_root_dir()?;
    let display_config_path = root.join("resources/display_config.ron");
    let assets_path = root.join("assets");
    // let config = DisplayConfig::load(&display_config_path);

    // let pipe = Pipeline::build().with_stage(
        // Stage::with_backbuffer()
            // .clear_target([1.0, 1.0, 1.0, 1.0], 1.0)
            // .with_pass(
                // DrawFlat2D::new()
                    // .with_transparency(ColorMask::all(), ALPHA, None)
            // ),
    // );

    let input_bundle = InputBundle::<StringBindings>::new()
        .with_bindings_from_file(root.join("resources/bindings_config.ron"))?;

    let game_data = GameDataBuilder::default()
        .with_bundle(WindowBundle::from_config_path(display_config_path))?
        .with_bundle(TransformBundle::new())?
        .with_bundle(input_bundle)?
        // .with_bundle(RenderBundle::new(pipe, Some(config))
            // .with_sprite_sheet_processor()
            // .with_sprite_visibility_sorting(&[])
        // )?
        .with(
            Processor::<SpriteSheet>::new(),
            "sprite_sheet_processor",
            &[],
        )
        .with(MarineAccelerationSystem, "marine_acceleration_system", &[])
        .with(AttackSystem, "attack_system", &["marine_acceleration_system"])
        .with(BulletCollisionSystem, "bullet_collision_system", &["marine_acceleration_system"])
        .with(BulletAnimationSystem, "bullet_animation_system", &["bullet_collision_system"])
        .with(BulletImpactAnimationSystem, "bullet_impact_animation_system", &["bullet_collision_system"])
        .with(MarineCollisionSystem, "marine_collision_system", &["marine_acceleration_system"])
        .with(MarineAnimationSystem, "marine_animation_system", &["marine_collision_system"])
        .with(CameraMotionSystem, "camera_motion_system", &["marine_collision_system"])
        .with_thread_local(RenderingSystem::<DefaultBackend, _>::new(
            ExampleGraph::default(),
        ));
    let mut game = Application::build(assets_path, states::PlayState::default())?
        // .with_frame_limit(
        //     FrameRateLimitStrategy::SleepAndYield(Duration::from_millis(2)),
        //     144,
        // )
        .build(game_data)?;

    game.run();

    Ok(())
}

#[derive(Default)]
struct ExampleGraph {
    dimensions: Option<ScreenDimensions>,
    surface_format: Option<Format>,
    dirty: bool,
}

impl GraphCreator<DefaultBackend> for ExampleGraph {
    fn rebuild(&mut self, res: &Resources) -> bool {
        // Rebuild when dimensions change, but wait until at least two frames have the same.
        let new_dimensions = res.try_fetch::<ScreenDimensions>();
        use std::ops::Deref;
        if self.dimensions.as_ref() != new_dimensions.as_ref().map(|d| d.deref()) {
            self.dirty = true;
            self.dimensions = new_dimensions.map(|d| d.clone());
            return false;
        }
        return self.dirty;
    }

    fn builder(
        &mut self,
        factory: &mut Factory<DefaultBackend>,
        res: &Resources,
    ) -> GraphBuilder<DefaultBackend, Resources> {
        use amethyst::renderer::rendy::{
            graph::present::PresentNode,
            hal::command::{ClearDepthStencil, ClearValue},
        };

        self.dirty = false;

        let window = <ReadExpect<'_, Arc<Window>>>::fetch(res);
        let surface = factory.create_surface(&window);
        // cache surface format to speed things up
        let surface_format = *self
            .surface_format
            .get_or_insert_with(|| factory.get_surface_format(&surface));
        let dimensions = self.dimensions.as_ref().unwrap();
        let window_kind =
            image::Kind::D2(dimensions.width() as u32, dimensions.height() as u32, 1, 1);

        let mut graph_builder = GraphBuilder::new();
        let color = graph_builder.create_image(
            window_kind,
            1,
            surface_format,
            Some(ClearValue::Color([0.34, 0.36, 0.52, 1.0].into())),
        );

        let depth = graph_builder.create_image(
            window_kind,
            1,
            Format::D32Sfloat,
            Some(ClearValue::DepthStencil(ClearDepthStencil(1.0, 0))),
        );

        let sprite = graph_builder.add_node(
            SubpassBuilder::new()
                .with_group(DrawFlat2DDesc::new().builder())
                .with_color(color)
                .with_depth_stencil(depth)
                .into_pass(),
        );

        let _present = graph_builder
            .add_node(PresentNode::builder(factory, surface, color).with_dependency(sprite));

        graph_builder
    }
}
