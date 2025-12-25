use nalgebra::Vector3;
use rapier3d::prelude::*;
use std::sync::Mutex;

struct GameState {
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    integration_parameters: IntegrationParameters,
    gravity: Vector3<f32>,

    camera_angle_x: f32,
    camera_angle_y: f32,
    camera_dist: f32,
    camera_target: Vector3<f32>,

    fire_cooldown: i32,
    block_cooldown: i32,
}

impl GameState {
    fn new() -> Self {
        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();

        // Ground
        let ground_size = 20.0;
        let ground_thickness = 1.0;
        let rigid_body = RigidBodyBuilder::fixed()
            .translation(Vector3::new(0.0, -ground_thickness, 0.0))
            .build();
        let collider = ColliderBuilder::cuboid(ground_size, ground_thickness, ground_size).build();
        let body_handle = rigid_body_set.insert(rigid_body);
        collider_set.insert_with_parent(collider, body_handle, &mut rigid_body_set);

        let mut state = Self {
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_body_set,
            collider_set,
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            integration_parameters: IntegrationParameters::default(),
            gravity: Vector3::new(0.0, -9.81, 0.0),

            camera_angle_x: 0.0,
            camera_angle_y: 0.5,
            camera_dist: 15.0,
            camera_target: Vector3::new(0.0, 2.0, 0.0),

            fire_cooldown: 0,
            block_cooldown: 0,
        };

        state.create_stack(3, 3);
        state
    }

    fn create_stack(&mut self, rows: i32, cols: i32) {
        let size = 1.0;
        let gap = 0.1;

        for y in 0..5 {
            for x in 0..rows {
                for z in 0..cols {
                    let pos_x = (x as f32 - rows as f32 / 2.0) * (size + gap);
                    let pos_y = y as f32 * (size + gap) + size / 2.0;
                    let pos_z = (z as f32 - cols as f32 / 2.0) * (size + gap);

                    self.create_cube(Vector3::new(pos_x, pos_y, pos_z));
                }
            }
        }
    }

    fn create_cube(&mut self, pos: Vector3<f32>) {
        let rigid_body = RigidBodyBuilder::dynamic().translation(pos).build();
        let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.5)
            .restitution(0.5)
            .build();
        let body_handle = self.rigid_body_set.insert(rigid_body);
        self.collider_set
            .insert_with_parent(collider, body_handle, &mut self.rigid_body_set);
    }

    fn fire_sphere(&mut self, pos: Vector3<f32>, dir: Vector3<f32>) {
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(pos)
            .linvel(dir * 20.0)
            .build();
        let collider = ColliderBuilder::ball(0.4)
            .restitution(0.7)
            .density(2.0)
            .build();
        let body_handle = self.rigid_body_set.insert(rigid_body);
        self.collider_set
            .insert_with_parent(collider, body_handle, &mut self.rigid_body_set);
    }

    fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }
}

static STATE: Mutex<Option<GameState>> = Mutex::new(None);

#[no_mangle]
pub extern "C" fn setup() {
    wasm96::graphics::set_size(640, 480);
    wasm96::graphics::set_3d(true);
    wasm96::graphics::font_register_spleen("spleen", 12);

    // Initialize Physics
    let mut state = STATE.lock().unwrap();
    *state = Some(GameState::new());

    // Create Meshes
    create_cube_mesh();
    create_sphere_mesh();
}

#[no_mangle]
pub extern "C" fn draw() {
    let mut state_guard = STATE.lock().unwrap();
    if let Some(state) = state_guard.as_mut() {
        // Physics Step
        state.step();

        // Input
        if wasm96::input::is_button_down(0, wasm96::Button::Up) {
            state.camera_target.z -= 0.1;
        }
        if wasm96::input::is_button_down(0, wasm96::Button::Down) {
            state.camera_target.z += 0.1;
        }
        if wasm96::input::is_button_down(0, wasm96::Button::Left) {
            state.camera_target.x -= 0.1;
        }
        if wasm96::input::is_button_down(0, wasm96::Button::Right) {
            state.camera_target.x += 0.1;
        }

        if wasm96::input::is_button_down(0, wasm96::Button::X) {
            state.camera_dist += 0.1;
        }
        if wasm96::input::is_button_down(0, wasm96::Button::Y) {
            state.camera_dist -= 0.1;
        }

        if wasm96::input::is_button_down(0, wasm96::Button::L1) {
            state.camera_angle_x -= 0.05;
        }
        if wasm96::input::is_button_down(0, wasm96::Button::R1) {
            state.camera_angle_x += 0.05;
        }

        // Calculate Camera Position
        let cam_x = state.camera_target.x
            + state.camera_dist * state.camera_angle_x.sin() * state.camera_angle_y.cos();
        let cam_y = state.camera_target.y + state.camera_dist * state.camera_angle_y.sin();
        let cam_z = state.camera_target.z
            + state.camera_dist * state.camera_angle_x.cos() * state.camera_angle_y.cos();
        let cam_pos = Vector3::new(cam_x, cam_y, cam_z);

        // Fire
        if wasm96::input::is_button_down(0, wasm96::Button::A) {
            if state.fire_cooldown <= 0 {
                let dir = (state.camera_target - cam_pos).normalize();
                state.fire_sphere(cam_pos + dir * 2.0, dir);
                state.fire_cooldown = 10;
            }
        }
        if state.fire_cooldown > 0 {
            state.fire_cooldown -= 1;
        }

        // Place Block
        if wasm96::input::is_button_down(0, wasm96::Button::B) {
            if state.block_cooldown <= 0 {
                state.create_cube(state.camera_target + Vector3::new(0.0, 5.0, 0.0));
                state.block_cooldown = 10;
            }
        }
        if state.block_cooldown > 0 {
            state.block_cooldown -= 1;
        }

        // Render
        wasm96::graphics::background(30, 30, 30);
        wasm96::graphics::camera_perspective(1.0, 640.0 / 480.0, 0.1, 100.0);
        wasm96::graphics::camera_look_at(
            (cam_pos.x, cam_pos.y, cam_pos.z),
            (
                state.camera_target.x,
                state.camera_target.y,
                state.camera_target.z,
            ),
            (0.0, 1.0, 0.0),
        );

        // Draw bodies
        for (_handle, body) in state.rigid_body_set.iter() {
            let pos = body.translation();
            let rot = body.rotation();
            let (roll, pitch, yaw) = rot.euler_angles();

            if let Some(collider_handle) = body.colliders().first() {
                if let Some(collider) = state.collider_set.get(*collider_handle) {
                    let shape = collider.shape();
                    if let Some(_ball) = shape.as_ball() {
                        wasm96::graphics::set_color(255, 100, 100, 255);
                        wasm96::graphics::mesh_draw(
                            "sphere",
                            (pos.x, pos.y, pos.z),
                            (roll, pitch, yaw),
                            (0.4, 0.4, 0.4),
                        );
                    } else if let Some(_cuboid) = shape.as_cuboid() {
                        if body.is_fixed() {
                            wasm96::graphics::set_color(100, 255, 100, 255);
                            wasm96::graphics::mesh_draw(
                                "cube",
                                (pos.x, pos.y, pos.z),
                                (roll, pitch, yaw),
                                (20.0, 1.0, 20.0),
                            );
                        } else {
                            wasm96::graphics::set_color(100, 100, 255, 255);
                            wasm96::graphics::mesh_draw(
                                "cube",
                                (pos.x, pos.y, pos.z),
                                (roll, pitch, yaw),
                                (0.5, 0.5, 0.5),
                            );
                        }
                    }
                }
            }
        }

        // Crosshair
        wasm96::graphics::set_color(255, 255, 255, 255);
        wasm96::graphics::line(320 - 5, 240, 320 + 5, 240);
        wasm96::graphics::line(320, 240 - 5, 320, 240 + 5);

        wasm96::graphics::text_key(10, 10, "spleen", "Rapier3D Demo");
        wasm96::graphics::text_key(10, 25, "spleen", "DPAD: Pan, X/Y: Zoom");
        wasm96::graphics::text_key(10, 40, "spleen", "L1/R1: Rotate, A: Fire, B: Drop Cube");
    }
}

fn create_cube_mesh() {
    let vertices: &[f32] = &[
        // Front face (Z+)
        -1.0, -1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0,
        1.0, 1.0, 1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0,
        // Back face (Z-)
        1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0, -1.0, -1.0, -1.0, 1.0, 0.0, 0.0, 0.0, -1.0, -1.0,
        1.0, -1.0, 1.0, 1.0, 0.0, 0.0, -1.0, 1.0, 1.0, -1.0, 0.0, 1.0, 0.0, 0.0, -1.0,
        // Top face (Y+)
        -1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0,
        -1.0, 1.0, 1.0, 0.0, 1.0, 0.0, -1.0, 1.0, -1.0, 0.0, 1.0, 0.0, 1.0, 0.0,
        // Bottom face (Y-)
        -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, -1.0, -1.0, 1.0, 0.0, 0.0, -1.0, 0.0, 1.0,
        -1.0, 1.0, 1.0, 1.0, 0.0, -1.0, 0.0, -1.0, -1.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0,
        // Right face (X+)
        1.0, -1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, -1.0, -1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0,
        -1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0,
        // Left face (X-)
        -1.0, -1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, -1.0, 1.0, 1.0, 0.0, -1.0, 0.0, 0.0, -1.0,
        1.0, 1.0, 1.0, 1.0, -1.0, 0.0, 0.0, -1.0, 1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 0.0,
    ];

    let indices: &[u32] = &[
        0, 1, 2, 0, 2, 3, // Front
        4, 5, 6, 4, 6, 7, // Back
        8, 9, 10, 8, 10, 11, // Top
        12, 13, 14, 12, 14, 15, // Bottom
        16, 17, 18, 16, 18, 19, // Right
        20, 21, 22, 20, 22, 23, // Left
    ];

    wasm96::graphics::mesh_create("cube", vertices, indices);
}

fn create_sphere_mesh() {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let rings = 8;
    let sectors = 8;
    let r = 1.0;

    for i in 0..=rings {
        let lat = std::f32::consts::PI * i as f32 / rings as f32;
        let y = r * lat.cos();
        let ring_radius = r * lat.sin();

        for j in 0..=sectors {
            let lon = 2.0 * std::f32::consts::PI * j as f32 / sectors as f32;
            let x = ring_radius * lon.cos();
            let z = ring_radius * lon.sin();

            // x, y, z, u, v, nx, ny, nz
            vertices.push(x);
            vertices.push(y);
            vertices.push(z);
            vertices.push(j as f32 / sectors as f32); // u
            vertices.push(i as f32 / rings as f32); // v
            vertices.push(x); // nx
            vertices.push(y); // ny
            vertices.push(z); // nz
        }
    }

    for i in 0..rings {
        for j in 0..sectors {
            let first = (i * (sectors + 1)) + j;
            let second = first + sectors + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    wasm96::graphics::mesh_create("sphere", &vertices, &indices);
}
