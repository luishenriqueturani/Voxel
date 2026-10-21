/// Malha triangular com atributos opcionais e uma textura.
#[derive(Clone, Debug)]
pub(crate) struct Surface {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub texture: Option<image::RgbaImage>,
}

impl Surface {
    pub(crate) fn triangles(&self) -> usize {
        self.indices.len() / 3
    }
}

pub(crate) fn face_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0; 3]; positions.len()];
    for face in indices.chunks_exact(3) {
        let Some(first) = face.first().copied().and_then(|index| at(positions, index)) else {
            continue;
        };
        let Some(second) = face.get(1).copied().and_then(|index| at(positions, index)) else {
            continue;
        };
        let Some(third) = face.get(2).copied().and_then(|index| at(positions, index)) else {
            continue;
        };
        let normal = cross(sub(second, first), sub(third, first));
        for index in face {
            if let Some(slot) = usize::try_from(*index)
                .ok()
                .filter(|slot| *slot < normals.len())
            {
                normals[slot] = add(normals[slot], normal);
            }
        }
    }
    for normal in &mut normals {
        *normal = unit(*normal);
    }
    normals
}

/// Centraliza em X e Z, põe o pivô no chão e deixa o maior lado em 1.
pub(crate) fn place_on_floor(positions: &mut [[f32; 3]]) {
    let Some(first) = positions.first().copied() else {
        return;
    };
    let mut min = first;
    let mut max = first;
    for position in positions.iter() {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    let span = (max[0] - min[0]).max(max[1] - min[1]).max(max[2] - min[2]);
    let scale = if span > f32::EPSILON { span } else { 1.0 };
    let center_x = (min[0] + max[0]) * 0.5;
    let center_z = (min[2] + max[2]) * 0.5;
    let floor = min[1];
    for position in positions.iter_mut() {
        position[0] = (position[0] - center_x) / scale;
        position[1] = (position[1] - floor) / scale;
        position[2] = (position[2] - center_z) / scale;
    }
}

fn at(positions: &[[f32; 3]], index: u32) -> Option<[f32; 3]> {
    usize::try_from(index)
        .ok()
        .and_then(|slot| positions.get(slot).copied())
}

fn sub(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn add(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn unit(value: [f32; 3]) -> [f32; 3] {
    let length = (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt();
    if length <= f32::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        [value[0] / length, value[1] / length, value[2] / length]
    }
}
