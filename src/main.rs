use bevy::{
    camera::ScalingMode, ecs::relationship::Relationship, prelude::*, window::{EnabledButtons, WindowResolution}
};

use bevy_spritesheet_animation::prelude::*;

const U_GAME_WIDTH: u32 = 1280;
const U_GAME_HEIGHT: u32 = 720;

const F_GAME_WIDTH: f32 = 1280.0;
const F_GAME_HEIGHT: f32 = 720.0;

#[derive(Component)]
struct Player;

#[derive(Resource, Default)]
struct CursorCoords {
    screen: Vec2,
    world: Vec2
}

#[derive(Component, Default)]
struct Movement {
    target: Vec2,
    velocity: Vec2,
    is_moving: bool
}

impl Movement {
    fn move_to(&mut self, target: Vec2) {
        self.target = target;
        self.is_moving = true;
    }

    fn stop(&mut self) {
        self.target = Vec2::ZERO;
        self.velocity = Vec2::ZERO;
        self.is_moving = false;
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(U_GAME_WIDTH, U_GAME_HEIGHT),
                        resizable: false,
                        enabled_buttons: EnabledButtons {
                            maximize: true,
                            minimize: true,
                            close: true,
                        },
                        ..default()
                    }),
                    ..default()
                })
                // .set(ImagePlugin::default_nearest())
        )
        .add_plugins(SpritesheetAnimationPlugin)
        .add_systems(Startup, (setup, spawn_player_unit))
        .add_systems(Update, (update, cursor_moved_system, camera_input_system, player_input_system, movement_system, engine_system))
        // .add_systems(Update, (update_system, handle_death_animation, camera_input_system, player_input_system, player_move_system))
        .run();
}

fn setup(mut cmds: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: F_GAME_WIDTH,
        height: F_GAME_HEIGHT,
    };

    cmds.spawn((Camera2d, Projection::Orthographic(projection)));

    cmds.init_resource::<CursorCoords>();
}

fn update(
) {
    
}

fn cursor_moved_system(
    mut events: MessageReader<CursorMoved>,
    q_camera: Single<(&Camera, &GlobalTransform)>,
    mut cursor: ResMut<CursorCoords>
) {
    let (cam, gt) = *q_camera;

    for event in events.read() {
        cursor.screen = event.position;
        // println!("cursor screen coords: {}", cursor.screen);

        if let Ok(world_coords) = cam.viewport_to_world_2d(gt, event.position) {
            cursor.world = world_coords;
            // println!("cursor world coords: {}", cursor.world);
        }
    }
}

fn camera_input_system(
    kb: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    camera: Single<&mut Transform, With<Camera>>,
) {
    const SPEED: f32 = 400.0;

    let mut t = camera.into_inner();

    if kb.pressed(KeyCode::KeyW) {
        t.translation.y += SPEED * time.delta_secs();
    }

    if kb.pressed(KeyCode::KeyA) {
        t.translation.x -= SPEED * time.delta_secs();
    }

    if kb.pressed(KeyCode::KeyS) {
        t.translation.y -= SPEED * time.delta_secs();
    }

    if kb.pressed(KeyCode::KeyD) {
        t.translation.x += SPEED * time.delta_secs();
    }
}

fn player_input_system(
    btns: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorCoords>,
    mut movement: Single<&mut Movement, With<Player>>
) {
    if btns.just_pressed(MouseButton::Right) {
        movement.move_to(cursor.world);
    }
}

fn movement_system(
    mut q_movement: Query<(&mut Transform, &mut Movement)>,
    time: Res<Time>
) {
    for (mut t, mut m) in q_movement.iter_mut() {
        if m.target == Vec2::ZERO {
            continue;
        }

        let direction = m.target - t.translation.truncate();

        if direction.length() < 2.0 {
            m.stop();
            continue;
        }

        let angle = direction.y.atan2(direction.x);
        t.rotation = Quat::from_rotation_z(angle - std::f32::consts::FRAC_PI_2);

        let direction = direction.normalize();        
        m.velocity = direction * 200.0 * time.delta_secs();

        t.translation.x += m.velocity.x;
        t.translation.y += m.velocity.y;
    }
}

fn create_atlas(
    x: u32,
    y: u32,
    columns: u32,
    rows: u32
) -> TextureAtlasLayout {
    return TextureAtlasLayout::from_grid(UVec2::new(x, y), columns, rows, None, None);
} 

fn create_animation(
    img: Handle<Image>,
    columns: usize,
    rows: usize,
    animation_repeat: AnimationRepeat
) -> Animation {
    return Spritesheet::new(&img, columns, rows)
        .create_animation()
        .add_row(0)
        .set_repetitions(animation_repeat)
        .build();
}

fn create_sprite(
    img: Handle<Image>,
    layout: Handle<TextureAtlasLayout>
) -> Sprite {
    return Sprite {
        image: img.into(),
        texture_atlas: Some(TextureAtlas {
            layout: layout,
            index: 0
        }),
        color: Color::WHITE,
        ..default()
    }
}

#[derive(Component)]
struct Unit;

#[derive(Component)]
struct Engine;

fn spawn_player_unit(
    mut cmds: Commands,
    assets: Res<AssetServer>,
    mut animations: ResMut<Assets<Animation>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>
) {
    let base_img = assets.load("unit_biomantes_scout_base.png");
    let base_layout = layouts.add(create_atlas(64, 64, 7, 1));
    let base_animation = animations.add(create_animation(base_img.clone(), 7, 1, AnimationRepeat::Loop));    
    
    let base_sprite = create_sprite(base_img, base_layout);
    let base_ssanimation = SpritesheetAnimation::new(base_animation);
    
    let engine_img = assets.load("unit_biomantes_scout_engine.png");
    let engine_layout = layouts.add(create_atlas(64, 64, 8, 1));
    let engine_animation = animations.add(create_animation(engine_img.clone(), 8, 1, AnimationRepeat::Loop));
    
    let engine_sprite = create_sprite(engine_img, engine_layout);
    let engine_ssanimation = SpritesheetAnimation::new(engine_animation);

    let transform = Transform::from_xyz(0.0, 0.0, 0.0);

    let entt = cmds.spawn((
        Player,
        Movement {
            target: Vec2::ZERO,
            velocity: Vec2::ZERO,
            is_moving: false
        },
        Unit,
        transform,
        base_sprite,
        base_ssanimation,
    )).id();

    cmds.spawn((
        engine_sprite,
        engine_ssanimation,
        transform,
        Visibility::Hidden
    )).set_parent_in_place(entt);
}

fn engine_system(
    mut q_engine: Query<(&mut Visibility, &ChildOf), With<Engine>>,
    q_movement: Query<&Movement>,
) {
    for (mut v, child_of) in q_engine.iter_mut() {
        if let Ok(m) = q_movement.get(child_of.get()) {
            *v = if m.is_moving {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}