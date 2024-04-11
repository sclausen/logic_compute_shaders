const hash_k1: u32 = 15823;
const hash_k2: u32 = 9737333;

fn get_offset2(i: i32) -> vec2<i32> {
    switch(i) {
            case 0: {
            return vec2<i32>(-1, 1);
        }
            case 1: {
            return vec2<i32>(0, 1);
        }
            case 2: {
            return vec2<i32>(1, 1);
        }
            case 3: {
            return vec2<i32>(-1, 0);
        }
            case 4: {
            return vec2<i32>(0, 0);
        }
            case 5: {
            return vec2<i32>(1, 0);
        }
            case 6: {
            return vec2<i32>(-1, -1);
        }
            case 7: {
            return vec2<i32>(0, -1);
        }
            case 8: {
            return vec2<i32>(1, -1);
        }
            default: {
            return vec2<i32>(0, 0);
        }
    }
}

fn get_offset(i: i32) -> vec2<i32> {
    switch (i) {
            case 0: {
            return vec2<i32>(-1, -1);
        }
            case 1: {
            return vec2<i32>(-1, 0);
        }
            case 2: {
            return vec2<i32>(-1, 1);
        }
            case 3: {
            return vec2<i32>(0, -1);
        }
            case 4: {
            return vec2<i32>(0, 0);
        }
            case 5: {
            return vec2<i32>(0, 1);
        }
            case 6: {
            return vec2<i32>(1, -1);
        }
            case 7: {
            return vec2<i32>(1, 0);
        }
            case 8: {
            return vec2<i32>(1, 1);
        }
            default: {
            return vec2<i32>(0, 0);
        }
    }
}

fn get_cell(position: vec2<f32>, radius: f32) -> vec2<i32> {
    return vec2<i32>(i32(floor(position.x / radius)), i32(floor(position.y / radius)));
}

fn hash_cell(cell: vec2<i32>) -> u32 {
    let cell_u = vec2<u32>(u32(cell.x), u32(cell.y));
    let a: u32 = cell_u.x * hash_k1;
    let b: u32 = cell_u.y * hash_k2;
    return (a + b);
}

fn key_from_hash(hash: u32, table_size: u32) -> u32 {
    return hash % table_size;
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> vec3<f32> {
    let c = (1.0 - abs(2.0 * l - 1.0)) * s;
    let x = c * (1.0 - abs(((h / 60.0) % 2.0) - 1.0));
    let m = l - c * 0.5;
    var rgb = vec3<f32>(0.0, 0.0, 0.0);

    if h < 60.0 {
        rgb = vec3<f32>(c, x, 0.0);
    } else if h < 120.0 {
        rgb = vec3<f32>(x, c, 0.0);
    } else if h < 180.0 {
        rgb = vec3<f32>(0.0, c, x);
    } else if h < 240.0 {
        rgb = vec3<f32>(0.0, x, c);
    } else if h < 300.0 {
        rgb = vec3<f32>(x, 0.0, c);
    } else if h < 360.0 {
        rgb = vec3<f32>(c, 0.0, x);
    }

    return rgb + vec3<f32>(m, m, m);
}

fn force(r: f32, a: f32) -> f32 {
    let beta: f32 = 0.3;
    if r < beta {
        return r / beta - 1.0;
    } else if beta < r && r < 1.0 {
        return a * (1.0 - abs(2.0 * r - 1.0 - beta) / (1.0 - beta));
    } else {
        return 0.0;
    }
}

fn get_wrapped_neighbor_cell(origin_cell: vec2<i32>, index: i32, width: i32, height: i32) -> vec2<i32> {
    let offset = get_offset(index);
    let neighbor_cell = vec2<i32>(origin_cell.x + offset.x, origin_cell.y + offset.y);
    let wrapped_x = (neighbor_cell.x + width) % width;
    let wrapped_y = (neighbor_cell.y + height) % height;
    return vec2<i32>(wrapped_x, wrapped_y);
}