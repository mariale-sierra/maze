pub const MAP_WIDTH: usize = 21;
pub const MAP_HEIGHT: usize = 15;

pub struct Maze {
    pub grid: Vec<Vec<char>>,
}

impl Maze {
    pub fn new_backrooms() -> Self {
        let mut grid = vec![vec!['.'; MAP_WIDTH]; MAP_HEIGHT];

        // bordes exteriores
        for x in 0..MAP_WIDTH {
            grid[0][x] = '1';
            grid[MAP_HEIGHT - 1][x] = '1';
        }
        for y in 0..MAP_HEIGHT {
            grid[y][0] = '1';
            grid[y][MAP_WIDTH - 1] = '1';
        }

        // pilares interiores, alternando el tipo de pared (colores/texturas distintas)
        let pillar_types = ['2', '3', '4'];
        let mut type_idx = 0;
        let mut y = 3;
        while y < MAP_HEIGHT - 3 {
            let mut x = 3;
            while x < MAP_WIDTH - 3 {
                grid[y][x] = pillar_types[type_idx % pillar_types.len()];
                type_idx += 1;
                x += 4;
            }
            y += 4;
        }

        // un pasillo interior para dar variedad
        for x in 6..15 {
            grid[7][x] = '1';
        }
        grid[7][10] = '.'; // hueco para poder pasar

        // "puerta" de meta en una esquina (color distinto)
        grid[MAP_HEIGHT - 2][MAP_WIDTH - 2] = 'g';

        Maze { grid }
    }

    pub fn is_wall_cell(&self, cx: i32, cy: i32) -> bool {
        if cx < 0 || cy < 0 || cy as usize >= self.grid.len() || cx as usize >= self.grid[0].len() {
            return true; // fuera del mapa cuenta como pared para no salirse
        }
        self.grid[cy as usize][cx as usize] != '.'
    }

    pub fn cell_at(&self, cx: i32, cy: i32) -> char {
        if cx < 0 || cy < 0 || cy as usize >= self.grid.len() || cx as usize >= self.grid[0].len() {
            return '1';
        }
        self.grid[cy as usize][cx as usize]
    }

    pub fn collides(&self, x: f32, y: f32, radius: f32) -> bool {
        let points = [
            (x - radius, y - radius),
            (x + radius, y - radius),
            (x - radius, y + radius),
            (x + radius, y + radius),
        ];
        for (px, py) in points.iter() {
            if self.is_wall_cell(px.floor() as i32, py.floor() as i32) {
                return true;
            }
        }
        false
    }

    pub fn width(&self) -> usize {
        self.grid[0].len()
    }

    pub fn height(&self) -> usize {
        self.grid.len()
    }
}

pub struct RayHit {
    pub perp_dist: f32,
    pub wall_char: char,
    pub wall_x: f32, // 0.0 a 1.0, posicion horizontal dentro de la pared golpeada
    pub side: i32,   // 0 = pared vertical, 1 = pared horizontal
}

/// Algoritmo DDA clasico (todo en "unidades de celda", 1.0 = una casilla del mapa).
pub fn cast_ray(maze: &Maze, pos_x: f32, pos_y: f32, angle: f32) -> RayHit {
    let dir_x = angle.cos();
    let dir_y = angle.sin();

    let mut map_x = pos_x.floor() as i32;
    let mut map_y = pos_y.floor() as i32;

    let delta_dist_x = if dir_x.abs() < 1e-6 { 1e30 } else { (1.0 / dir_x).abs() };
    let delta_dist_y = if dir_y.abs() < 1e-6 { 1e30 } else { (1.0 / dir_y).abs() };

    let (step_x, mut side_dist_x) = if dir_x < 0.0 {
        (-1, (pos_x - map_x as f32) * delta_dist_x)
    } else {
        (1, (map_x as f32 + 1.0 - pos_x) * delta_dist_x)
    };

    let (step_y, mut side_dist_y) = if dir_y < 0.0 {
        (-1, (pos_y - map_y as f32) * delta_dist_y)
    } else {
        (1, (map_y as f32 + 1.0 - pos_y) * delta_dist_y)
    };

    let mut side = 0;
    let mut hit_char = '1';

    for _ in 0..500 {
        if side_dist_x < side_dist_y {
            side_dist_x += delta_dist_x;
            map_x += step_x;
            side = 0;
        } else {
            side_dist_y += delta_dist_y;
            map_y += step_y;
            side = 1;
        }

        let c = maze.cell_at(map_x, map_y);
        if c != '.' {
            hit_char = c;
            break;
        }
    }

    let perp_dist = if side == 0 {
        (side_dist_x - delta_dist_x).max(0.0001)
    } else {
        (side_dist_y - delta_dist_y).max(0.0001)
    };

    let wall_x = if side == 0 {
        pos_y + perp_dist * dir_y
    } else {
        pos_x + perp_dist * dir_x
    };
    let wall_x = wall_x - wall_x.floor();

    RayHit {
        perp_dist,
        wall_char: hit_char,
        wall_x,
        side,
    }
}