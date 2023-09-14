use glam::{IVec2, Vec2, Vec3};

// Fluid sim based on https://github.com/matthias-research/pages/blob/master/tenMinutePhysics/18-flip.html

#[derive(Debug)]
pub struct FluidScene {
    pub width: f32,
    pub height: f32,
    tank_width: f32,
    tank_height: f32,
    gravity: Vec2,
    dt: f32,
    flip_ratio: f32,
    num_pressure_iters: usize,
    num_particle_iters: usize,
    over_relaxation: f32,
    compensate_drift: bool,
    paused: bool,
    pub obstacle_pos: Vec2,
    pub obstacle_radius: f32,
    obstacle_vel: Vec2,
    pub show_particles: bool,
    pub show_grid: bool,
    pub fluid: FluidSim,
}

impl FluidScene {
    pub fn new(width: f32, height: f32) -> FluidScene {
        let res = 100.0;
        let sim_height = 3.0;
        let c_scale = height / sim_height;
        let sim_width = width / c_scale;
        let tank_height = 1.0 * sim_height;
        let tank_width = 1.0 * sim_width;
        let h = tank_height / res;
        let rel_water_height = 0.8;
        let rel_water_width = 0.6;

        // particle radius w.r.t. cell size
        let r = 0.3 * h;
        let dx = 2.0 * r;
        let dy = 3.0_f32.sqrt() / 2.0 * dx;
        let num_x = ((rel_water_width * tank_width - 2.0 * h - 2.0 * r) / dx).floor() as usize;
        let num_y = ((rel_water_height * tank_height - 2.0 * h - 2.0 * r) / dy).floor() as usize;
        let num_particles = num_x * num_y;
        let mut fluid = FluidSim::new(tank_width, tank_height, h, r, num_particles);
        // Setup particles
        let mut i = 0;
        for x in 0..num_x {
            for y in 0..num_y {
                fluid.particle_pos[i].x = h + r + dx * x as f32 + if y % 2 == 0 { 0.0 } else { r };
                fluid.particle_pos[i].y = h + r + dy * y as f32;
                i += 1;
            }
        }

        // Setup grid cells
        for x in 0..fluid.f_num_x {
            for y in 0..fluid.f_num_y {
                // fluid
                let mut s = 1.0;
                if x == 0 || x == fluid.f_num_x - 1 || y == 0 || y == fluid.f_num_y - 1 {
                    // solid
                    s = 0.0;
                }
                fluid.s[x * fluid.f_num_y + y] = s;
            }
        }
        let obstacle_radius = 0.15;

        FluidScene {
            width,
            height,
            tank_width,
            tank_height,
            gravity: Vec2::new(0.0, -9.81),
            dt: 1.0 / 60.0,
            flip_ratio: 0.9,
            num_pressure_iters: 50,
            num_particle_iters: 2,
            over_relaxation: 1.9,
            compensate_drift: true,
            paused: true,
            obstacle_pos: Vec2::new(-obstacle_radius, -obstacle_radius),
            obstacle_vel: Vec2::ZERO,
            show_particles: true,
            show_grid: false,
            fluid,
            obstacle_radius,
        }
    }

    pub fn reset(&mut self) {
        *self = FluidScene::new(self.width, self.height);
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn toggle_grid(&mut self) {
        self.show_grid = !self.show_grid;
    }

    pub fn toggle_particles(&mut self) {
        self.show_particles = !self.show_particles;
    }

    pub fn toggle_gravity(&mut self) {
        if self.gravity == Vec2::ZERO {
            self.gravity = Vec2::new(0.0, -9.81);
        } else {
            self.gravity = Vec2::ZERO;
        }
    }

    pub fn render_pos(&self, pos: Vec2) -> Vec2 {
        pos * Vec2::new(self.width / self.tank_width, self.height / self.tank_height)
            - Vec2::new(self.width * 0.5, self.height * 0.5)
    }

    pub fn render_radius(&self) -> f32 {
        self.fluid.particle_radius * self.width / self.tank_width
    }

    pub fn render_cell_size(&self) -> f32 {
        self.fluid.h * self.width / self.tank_width
    }

    pub fn render_obstacle_radius(&self) -> f32 {
        self.obstacle_radius * self.width / self.tank_width
    }

    pub fn simulate(&mut self) {
        if !self.paused {
            self.fluid.simulate(
                self.dt,
                self.gravity,
                self.flip_ratio,
                self.num_pressure_iters,
                self.num_particle_iters,
                self.over_relaxation,
                self.compensate_drift,
                self.obstacle_pos,
                self.obstacle_vel,
                self.obstacle_radius,
            );
        }
    }

    pub fn drag(&mut self, cursor_world_pos: Vec2, is_start: bool) {
        let canvas_world_pos = cursor_world_pos + Vec2::new(self.width / 2.0, self.height / 2.0);
        let c_scale = self.width / self.tank_width;
        let tank_pos = canvas_world_pos / c_scale;
        self.set_obstacle(tank_pos, is_start);
    }

    pub fn end_drag(&mut self) {
        self.obstacle_vel = Vec2::ZERO;
    }

    pub fn set_obstacle(&mut self, pos: Vec2, reset: bool) {
        let mut v = Vec2::ZERO;

        let pos = pos.clamp(
            Vec2::new(
                self.obstacle_radius + self.fluid.h,
                self.obstacle_radius + self.fluid.h,
            ),
            Vec2::new(
                self.tank_width - self.obstacle_radius - self.fluid.h,
                self.tank_height - self.obstacle_radius - self.fluid.h,
            ),
        );
        if !reset {
            v = (pos - self.obstacle_pos) / self.dt;
        }

        self.obstacle_pos = pos;
        self.obstacle_vel = v;
    }
}

#[derive(Debug)]
pub struct FluidSim {
    // Fluid
    pub h: f32,
    f_inv_spacing: f32,
    pub f_num_x: usize,
    pub f_num_y: usize,
    pub f_num_cells: usize,
    u: Vec<f32>,
    v: Vec<f32>,
    du: Vec<f32>,
    dv: Vec<f32>,
    prev_u: Vec<f32>,
    prev_v: Vec<f32>,
    s: Vec<f32>,
    cell_type: Vec<CellType>,
    pub cell_color: Vec<Vec3>,
    // Particles
    p_inv_spacing: f32,
    pub particle_radius: f32,
    pub num_particles: usize,
    pub particle_pos: Vec<Vec2>,
    pub particle_color: Vec<Vec3>,
    particle_vel: Vec<Vec2>,
    particle_density: Vec<f32>,
    particle_rest_density: f32,
    p_num_x: usize,
    p_num_y: usize,
    particle_num_cells: usize,
    num_cell_particles: Vec<usize>,
    first_cell_particle: Vec<usize>,
    cell_particle_ids: Vec<usize>,
}

impl FluidSim {
    pub fn new(
        width: f32,
        height: f32,
        spacing: f32,
        particle_radius: f32,
        num_particles: usize,
    ) -> FluidSim {
        let fluid_num_x = (width / spacing).floor() as usize + 1;
        let fluid_num_y = (height / spacing).floor() as usize;
        let h = (width / fluid_num_x as f32).max(height / fluid_num_y as f32);
        let fluid_inv_spacing = 1.0 / h;
        let fluid_num_cells = fluid_num_x * fluid_num_y;
        let particle_inv_spacing = 1.0 / (2.2 * particle_radius);
        let particle_num_x = (width * particle_inv_spacing).floor() as usize + 1;
        let particle_num_y = (height * particle_inv_spacing).floor() as usize + 1;
        let particle_num_cells = particle_num_x * particle_num_y;
        let mut particle_color = vec![Vec3::ZERO; 3 * num_particles];
        for c in particle_color.iter_mut() {
            c.z = 1.0;
        }
        FluidSim {
            h,
            f_inv_spacing: fluid_inv_spacing,
            f_num_x: fluid_num_x,
            f_num_y: fluid_num_y,
            f_num_cells: fluid_num_cells,
            u: vec![0.0; fluid_num_cells],
            v: vec![0.0; fluid_num_cells],
            du: vec![0.0; fluid_num_cells],
            dv: vec![0.0; fluid_num_cells],
            prev_u: vec![0.0; fluid_num_cells],
            prev_v: vec![0.0; fluid_num_cells],
            s: vec![0.0; fluid_num_cells],
            cell_type: vec![CellType::Air; fluid_num_cells],
            cell_color: vec![Vec3::ZERO; fluid_num_cells],
            p_inv_spacing: particle_inv_spacing,
            particle_radius,
            num_particles,
            particle_pos: vec![Vec2::ZERO; num_particles],
            particle_color,
            particle_vel: vec![Vec2::ZERO; num_particles],
            particle_density: vec![0.0; fluid_num_cells],
            particle_rest_density: 0.0,
            p_num_x: particle_num_x,
            p_num_y: particle_num_y,
            particle_num_cells,
            num_cell_particles: vec![0; particle_num_cells],
            first_cell_particle: vec![0; particle_num_cells + 1],
            cell_particle_ids: vec![0; num_particles],
        }
    }

    pub fn simulate(
        &mut self,
        dt: f32,
        gravity: Vec2,
        flip_ratio: f32,
        num_pressure_iters: usize,
        num_particle_iters: usize,
        over_relaxation: f32,
        compensate_drift: bool,
        obstacle_pos: Vec2,
        obstacle_vel: Vec2,
        obstacle_radius: f32,
    ) {
        let sub_steps = 1;
        let sdt = dt / sub_steps as f32;

        for _ in 0..sub_steps {
            self.integrate_particles(sdt, gravity);
            self.push_particles_apart(num_particle_iters);
            self.handle_particle_collisions(obstacle_pos, obstacle_vel, obstacle_radius);
            self.transfer_velocities(true, flip_ratio);
            self.update_particle_density();
            self.solve_incompressibility(num_pressure_iters, over_relaxation, compensate_drift);
            self.transfer_velocities(false, flip_ratio);
        }

        self.update_particle_colors();
        self.update_cell_colors();
    }

    fn integrate_particles(&mut self, sdt: f32, gravity: Vec2) {
        for i in 0..self.num_particles {
            self.particle_vel[i] += gravity * sdt;
            let vel = self.particle_vel[i];
            self.particle_pos[i] += vel * sdt;
        }
    }

    fn push_particles_apart(&mut self, num_particle_iters: usize) {
        // count particles per cell
        self.num_cell_particles.fill(0);
        for i in 0..self.num_particles {
            let pos = self.particle_pos[i];
            let cell_number =
                pos_to_cell_index(pos, self.p_inv_spacing, self.p_num_x, self.p_num_y);
            self.num_cell_particles[cell_number] += 1;
        }

        // partial sums
        let mut first = 0;
        for i in 0..self.particle_num_cells {
            first += self.num_cell_particles[i];
            self.first_cell_particle[i] = first;
        }
        self.first_cell_particle[self.particle_num_cells] = first;

        // fill particles into cells

        for i in 0..self.num_particles {
            let pos = self.particle_pos[i];
            let cell_number =
                pos_to_cell_index(pos, self.p_inv_spacing, self.p_num_x, self.p_num_y);
            self.first_cell_particle[cell_number] -= 1;
            self.cell_particle_ids[self.first_cell_particle[cell_number]] = i;
        }
        // push particles apart
        let min_dist = 2.0 * self.particle_radius;
        let min_dist2 = min_dist * min_dist;

        for _ in 0..num_particle_iters {
            for i in 0..self.num_particles {
                let pos = self.particle_pos[i];
                let pxi = (pos.x * self.p_inv_spacing).floor() as i32;
                let pyi = (pos.y * self.p_inv_spacing).floor() as i32;
                let x0 = (pxi - 1).clamp(0, self.p_num_x as i32 - 1) as usize;
                let y0 = (pyi - 1).clamp(0, self.p_num_y as i32 - 1) as usize;
                let x1 = (pxi + 1).clamp(0, self.p_num_x as i32 - 1) as usize;
                let y1 = (pyi + 1).clamp(0, self.p_num_y as i32 - 1) as usize;
                for xi in x0..=x1 {
                    for yi in y0..=y1 {
                        let cell_number = xi * self.p_num_y + yi;
                        let first = self.first_cell_particle[cell_number];
                        let last = self.first_cell_particle[cell_number + 1];
                        for j in first..last {
                            let id = self.cell_particle_ids[j];
                            if id == i {
                                continue;
                            }
                            let q = self.particle_pos[id];
                            let mut d = q - pos;
                            let d2 = d.x * d.x + d.y * d.y;
                            if d2 > min_dist2 || d2 == 0.0 {
                                continue;
                            }
                            let dsqrt = d2.sqrt();
                            let s = 0.5 * (min_dist - dsqrt) / dsqrt;
                            d *= s;
                            self.particle_pos[i] -= d;
                            self.particle_pos[id] += d;
                        }
                    }
                }
            }
        }
    }

    fn handle_particle_collisions(
        &mut self,
        obstacle_pos: Vec2,
        obstacle_vel: Vec2,
        obstacle_radius: f32,
    ) {
        let h = 1.0 / self.f_inv_spacing;
        let r = self.particle_radius;
        let min_x = h + r;
        let max_x = (self.f_num_x - 1) as f32 * h - r;
        let min_y = h + r;
        let max_y = (self.f_num_y - 1) as f32 * h - r;

        let min_dist = obstacle_radius + r;
        let min_dist2 = min_dist * min_dist;

        for i in 0..self.num_particles {
            let mut pos = self.particle_pos[i];
            let od = pos - obstacle_pos;
            let d2 = od.x * od.x + od.y * od.y;
            if d2 < min_dist2 {
                self.particle_vel[i] = obstacle_vel;
            }

            if pos.x < min_x {
                pos.x = min_x;
                self.particle_vel[i].x = 0.0;
            }
            if pos.x > max_x {
                pos.x = max_x;
                self.particle_vel[i].x = 0.0;
            }
            if pos.y < min_y {
                pos.y = min_y;
                self.particle_vel[i].y = 0.0;
            }
            if pos.y > max_y {
                pos.y = max_y;
                self.particle_vel[i].y = 0.0;
            }
            self.particle_pos[i] = pos;
        }
    }

    fn transfer_velocities(&mut self, to_grid: bool, flip_ratio: f32) {
        let n = self.f_num_y;
        let h = self.h;
        let h1 = self.f_inv_spacing;
        let h2 = 0.5 * h;

        if to_grid {
            self.prev_u = self.u.clone();
            self.prev_v = self.v.clone();

            self.u.fill(0.0);
            self.v.fill(0.0);
            self.du.fill(0.0);
            self.dv.fill(0.0);

            for i in 0..self.f_num_cells {
                self.cell_type[i] = if self.s[i] == 0.0 {
                    CellType::Solid
                } else {
                    CellType::Air
                };
            }

            for i in 0..self.num_particles {
                let pos = self.particle_pos[i];
                let cell_number =
                    pos_to_cell_index(pos, self.f_inv_spacing, self.f_num_x, self.f_num_y);
                let cell_type = &mut self.cell_type[cell_number];
                if *cell_type == CellType::Air {
                    *cell_type = CellType::Fluid;
                }
            }
        }

        for component in 0..2 {
            let (dx, dy, f, prev_f, d, offset) = if component == 0 {
                (0.0, h2, &mut self.u, &self.prev_u, &mut self.du, n)
            } else {
                (h2, 0.0, &mut self.v, &self.prev_v, &mut self.dv, 1)
            };
            for i in 0..self.num_particles {
                let pos = self.particle_pos[i];
                let x = pos.x.clamp(h, (self.f_num_x - 1) as f32 * h);
                let y = pos.y.clamp(h, (self.f_num_y - 1) as f32 * h);

                let x0 = (((x - dx) * h1).floor() as i32).min(self.f_num_x as i32 - 2);
                let tx = ((x - dx) - x0 as f32 * h) * h1;
                let x1 = (x0 + 1).min(self.f_num_x as i32 - 2);

                let y0 = (((y - dy) * h1).floor() as i32).min(self.f_num_y as i32 - 2);
                let ty = ((y - dy) - y0 as f32 * h) * h1;
                let y1 = (y0 + 1).min(self.f_num_y as i32 - 2);

                let sx = 1.0 - tx;
                let sy = 1.0 - ty;

                let d0 = sx * sy;
                let d1 = tx * sy;
                let d2 = tx * ty;
                let d3 = sx * ty;

                let nr0 = x0 as usize * n + y0 as usize;
                let nr1 = x1 as usize * n + y0 as usize;
                let nr2 = x1 as usize * n + y1 as usize;
                let nr3 = x0 as usize * n + y1 as usize;

                let v = if component == 0 {
                    &mut self.particle_vel[i].x
                } else {
                    &mut self.particle_vel[i].y
                };
                if to_grid {
                    f[nr0] += *v * d0;
                    f[nr1] += *v * d1;
                    f[nr2] += *v * d2;
                    f[nr3] += *v * d3;
                    d[nr0] += d0;
                    d[nr1] += d1;
                    d[nr2] += d2;
                    d[nr3] += d3;
                } else {
                    let valid0 = if !self.cell_type[nr0].is_air()
                        || !self.cell_type[nr0 - offset].is_air()
                    {
                        1.0
                    } else {
                        0.0
                    };
                    let valid1 = if !self.cell_type[nr1].is_air()
                        || !self.cell_type[nr1 - offset].is_air()
                    {
                        1.0
                    } else {
                        0.0
                    };
                    let valid2 = if !self.cell_type[nr2].is_air()
                        || !self.cell_type[nr2 - offset].is_air()
                    {
                        1.0
                    } else {
                        0.0
                    };
                    let valid3 = if !self.cell_type[nr3].is_air()
                        || !self.cell_type[nr3 - offset].is_air()
                    {
                        1.0
                    } else {
                        0.0
                    };
                    let d = valid0 * d0 + valid1 * d1 + valid2 * d2 + valid3 * d3;
                    if d > 0.0 {
                        let pic_v = (valid0 * d0 * f[nr0]
                            + valid1 * d1 * f[nr1]
                            + valid2 * d2 * f[nr2]
                            + valid3 * d3 * f[nr3])
                            / d;
                        let corr = (valid0 * d0 * (f[nr0] - prev_f[nr0])
                            + valid1 * d1 * (f[nr1] - prev_f[nr1])
                            + valid2 * d2 * (f[nr2] - prev_f[nr2])
                            + valid3 * d3 * (f[nr3] - prev_f[nr3]))
                            / d;
                        let flip_v = *v + corr;
                        *v = (1.0 - flip_ratio) * pic_v + flip_ratio * flip_v;
                    }
                }
            }

            if to_grid {
                for i in 0..f.len() {
                    if d[i] > 0.0 {
                        f[i] /= d[i];
                    }
                }
            }
        }
        if to_grid {
            for i in 0..self.f_num_x {
                for j in 0..self.f_num_y {
                    let solid = self.cell_type[i * n + j] == CellType::Solid;
                    if solid {
                        self.u[i * n + j] = self.prev_u[i * n + j];
                        self.v[i * n + j] = self.prev_v[i * n + j];
                    } else {
                        if i > 0 && self.cell_type[(i - 1) * n + j] == CellType::Solid {
                            self.u[i * n + j] = self.prev_u[i * n + j];
                        }
                        if j > 0 && self.cell_type[i * n + j - 1] == CellType::Solid {
                            self.v[i * n + j] = self.prev_v[i * n + j];
                        }
                    }
                }
            }
        }
    }

    fn update_particle_density(&mut self) {
        let n = self.f_num_y;
        let h = self.h;
        let h1 = self.f_inv_spacing;
        let h2 = 0.5 * h;
        let d = &mut self.particle_density;
        d.fill(0.0);

        for i in 0..self.num_particles {
            let pos = self.particle_pos[i];

            let x = pos.x.clamp(h, (self.f_num_x - 1) as f32 * h);
            let y = pos.y.clamp(h, (self.f_num_y - 1) as f32 * h);

            let x0 = ((x - h2) * h1).floor() as i32;
            let tx = ((x - h2) - x0 as f32 * h) * h1;
            let x1 = (x0 + 1).min(self.f_num_x as i32 - 2);

            let y0 = ((y - h2) * h1).floor() as i32;
            let ty = ((y - h2) - y0 as f32 * h) * h1;
            let y1 = (y0 + 1).min(self.f_num_y as i32 - 2);

            let sx = 1.0 - tx;
            let sy = 1.0 - ty;

            if x0 < self.f_num_x as i32 && y0 < self.f_num_y as i32 {
                d[x0 as usize * n + y0 as usize] += sx * sy;
            }
            if x1 < self.f_num_x as i32 && y0 < self.f_num_y as i32 {
                d[x1 as usize * n + y0 as usize] += tx * sy;
            }
            if x1 < self.f_num_x as i32 && y1 < self.f_num_y as i32 {
                d[x1 as usize * n + y1 as usize] += tx * ty;
            }
            if x0 < self.f_num_x as i32 && y1 < self.f_num_y as i32 {
                d[x0 as usize * n + y1 as usize] += sx * ty;
            }
        }

        if self.particle_rest_density == 0.0 {
            let mut sum = 0.0;
            let mut num_fluid_cells = 0.0;
            for i in 0..self.f_num_cells {
                if self.cell_type[i] == CellType::Fluid {
                    sum += d[i];
                    num_fluid_cells += 1.0;
                }
            }

            if num_fluid_cells > 0.0 {
                self.particle_rest_density = sum / num_fluid_cells;
            }
        }
    }

    fn solve_incompressibility(
        &mut self,
        num_pressure_iters: usize,
        over_relaxation: f32,
        compensate_drift: bool,
    ) {
        self.prev_u = self.u.clone();
        self.prev_v = self.v.clone();
        let n = self.f_num_y;

        for _ in 0..num_pressure_iters {
            for i in 1..(self.f_num_x - 1) {
                for j in 1..(self.f_num_y - 1) {
                    if self.cell_type[i * n + j] != CellType::Fluid {
                        continue;
                    }
                    let center = i * n + j;
                    let left = (i - 1) * n + j;
                    let right = (i + 1) * n + j;
                    let bottom = i * n + j - 1;
                    let top = i * n + j + 1;

                    let sx0 = self.s[left];
                    let sx1 = self.s[right];
                    let sy0 = self.s[bottom];
                    let sy1 = self.s[top];
                    let s = sx0 + sx1 + sy0 + sy1;
                    if s == 0.0 {
                        continue;
                    }
                    let mut div = self.u[right] - self.u[center] + self.v[top] - self.v[center];
                    if self.particle_rest_density > 0.0 && compensate_drift {
                        let k = 1.0;
                        let compression =
                            self.particle_density[i * n + j] - self.particle_rest_density;
                        if compression > 0.0 {
                            div = div - k * compression;
                        }
                    }
                    let p = (-div / s) * over_relaxation;
                    self.u[center] -= sx0 * p;
                    self.u[right] += sx1 * p;
                    self.v[center] -= sy0 * p;
                    self.v[top] += sy1 * p;
                }
            }
        }
    }

    fn update_particle_colors(&mut self) {
        let d0 = self.particle_rest_density;
        for i in 0..self.num_particles {
            let pos = self.particle_pos[i];
            let s = 0.01;
            let color = &mut self.particle_color[i];
            color.x = (color.x - s).clamp(0.0, 1.0);
            color.y = (color.y - s).clamp(0.0, 1.0);
            color.z = (color.z + s).clamp(0.0, 1.0);

            if d0 > 0.0 {
                let cell_number =
                    pos_to_cell_index(pos, self.f_inv_spacing, self.f_num_x, self.f_num_y);
                let rel_density = self.particle_density[cell_number] / d0;
                if rel_density < 0.7 {
                    let s = 0.8;
                    color.x = s;
                    color.y = s;
                    color.z = 1.0;
                }
            }
        }
    }

    fn update_cell_colors(&mut self) {
        self.cell_color.fill(Vec3::ZERO);
        for i in 0..self.f_num_cells {
            if self.cell_type[i] == CellType::Solid {
                let color = &mut self.cell_color[i];
                color.x = 0.5;
                color.y = 0.5;
                color.z = 0.5;
            } else if self.cell_type[i] == CellType::Fluid {
                let mut d = self.particle_density[i];
                if self.particle_rest_density > 0.0 {
                    d /= self.particle_rest_density;
                }
                self.set_sci_color(i, d, 0.0, 2.0);
            }
        }
    }

    fn set_sci_color(&mut self, cell_number: usize, density: f32, min: f32, max: f32) {
        let val = density.max(min).min(max - 0.0001);
        let d = max - min;
        let val = if d == 0.0 { 0.5 } else { (val - min) / d };
        let m = 0.25;
        let num = (val / m).floor();
        let s = (val - num * m) / m;
        let mut r = 0.0;
        let mut g = 0.0;
        let mut b = 0.0;
        match num as i32 {
            0 => {
                r = 0.0;
                g = s;
                b = 1.0;
            }
            1 => {
                r = 0.0;
                g = 1.0;
                b = 1.0 - s;
            }
            2 => {
                r = s;
                g = 1.0;
                b = 0.0;
            }
            3 => {
                r = 1.0;
                g = 1.0 - s;
                b = 0.0;
            }
            _ => (),
        }
        let color = &mut self.cell_color[cell_number];
        color.x = r;
        color.y = g;
        color.z = b;
    }
}

fn pos_to_cell_pos(pos: Vec2, inv_spacing: f32, num_x: usize, num_y: usize) -> IVec2 {
    (pos * inv_spacing).floor().as_ivec2().clamp(
        IVec2::new(0, 0),
        IVec2::new(num_x as i32 - 1, num_y as i32 - 1),
    )
}

fn pos_to_cell_index(pos: Vec2, inv_spacing: f32, num_x: usize, num_y: usize) -> usize {
    let xyi = pos_to_cell_pos(pos, inv_spacing, num_x, num_y);
    xyi.x as usize * num_y + xyi.y as usize
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[repr(u32)]
pub enum CellType {
    Fluid,
    Air,
    Solid,
}

impl CellType {
    pub fn is_air(&self) -> bool {
        *self == CellType::Air
    }
}
