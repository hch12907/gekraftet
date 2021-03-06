use cgmath::{ Point3, Point2, Vector3 };
use crate::RGBA;
use super::{ Face, Mesh, Texture, Vertex };
//use rand::random;

const LIGHTING: [u32; 6] = [
    3, // back
    4, // right
    5, // top
    3, // front
    4, // left
    1, // bottom
];

const LIGHTING_VERT: [u32; 8] = [
    LIGHTING[0],
    LIGHTING[4],
    LIGHTING[1],
    LIGHTING[3],
    LIGHTING[5],
    LIGHTING[3],
    LIGHTING[2],
    5, // 8th vert is not involved in lighting
];

pub struct MeshBuilder {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    textures: Vec<Texture>,
}

impl MeshBuilder {
    pub fn new() -> MeshBuilder {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            textures: Vec::new(),
        }
    }

    pub fn create_cuboid(length: Vector3<f32>, origin: Point3<f32>, faces: Face) -> Mesh {
        if faces == Face::empty() {
            return MeshBuilder::new().build()
        };
        
        // It is typical to see a section with >=24 vertices. Rounded up to 32.
        let mut actual_indices = Vec::with_capacity(32);
        let mut mapped_indices = [std::u32::MAX; 128];
        let mut added_vertices = Vec::with_capacity(32);

        let halved = length * 0.5;
        let create_vertex = |x, y, z, lighting| {
            let color = {
                //let origin = origin * 0.026315; // (1 / 38.0)
                //let (x, y, z) = (origin.x(), origin.y(), origin.z());
                RGBA::new(0.9, 0.9, 0.9, 1.0)
            };

            Vertex::new(
                Point3::<f32>::new(x + origin.x, y + origin.y, z + origin.z),
                color,
                //RGBA::new(0.8, 0.8, 0.8, 1.0),
                Point2::<f32>::new(lighting as f32 * 0.2, 0.0)
            )
        };

        let mut add_face = |indices: [usize; 6]| {
            for &index in indices.iter() {
                if mapped_indices[index] == std::u32::MAX {
                    let vertex = match index {
                        0 => create_vertex(-halved.x, -halved.y, -halved.z, LIGHTING_VERT[0]), // index 0
                        1 => create_vertex(-halved.x,  halved.y, -halved.z, LIGHTING_VERT[1]), // index 1
                        2 => create_vertex( halved.x,  halved.y, -halved.z, LIGHTING_VERT[2]), // index 2
                        3 => create_vertex( halved.x, -halved.y, -halved.z, LIGHTING_VERT[3]), // index 3
                        4 => create_vertex(-halved.x, -halved.y,  halved.z, LIGHTING_VERT[4]), // index 4
                        5 => create_vertex(-halved.x,  halved.y,  halved.z, LIGHTING_VERT[5]), // index 5
                        6 => create_vertex( halved.x,  halved.y,  halved.z, LIGHTING_VERT[6]), // index 6
                        7 => create_vertex( halved.x, -halved.y,  halved.z, LIGHTING_VERT[7]), // index 7
                        _ => unreachable!(),
                    };
                    actual_indices.push(added_vertices.len() as u32);
                    mapped_indices[index] = added_vertices.len() as u32;
                    added_vertices.push(vertex);
                } else {
                    actual_indices.push(mapped_indices[index]);
                }
            }
        };

        if faces.intersects(Face::BACK) {
            add_face([1, 3, 0, 1, 2, 3]);
        };

        if faces.intersects(Face::RIGHT) {
            add_face([7, 3, 2, 6, 7, 2]);
        };
            
        if faces.intersects(Face::TOP) {
            add_face([1, 5, 6, 2, 1, 6]);
        }

        if faces.intersects(Face::FRONT) {
            add_face([4, 7, 5, 7, 6, 5]);
        }

        if faces.intersects(Face::LEFT) {
            add_face([0, 4, 1, 4, 5, 1]);
        }

        if faces.intersects(Face::BOTTOM) {
            add_face([3, 7, 4, 0, 3, 4]);
        }

        let builder = Self {
            vertices: added_vertices,
            indices: actual_indices,
            textures: Vec::new(),
        };

        builder.build()
    }

    pub fn create_cube(length: f32, origin: Point3<f32>, faces: Face) -> Mesh {
        Self::create_cuboid(
            Vector3::<f32>::new(length, length, length), 
            origin,
            faces
        )
    }

    pub fn add_vertex(mut self, vert: Vertex) -> Self {
        self.vertices.push(vert);
        self
    }

    pub fn add_index(mut self, index: u32) -> Self {
        self.indices.push(index);
        self
    }

    pub fn add_texture(mut self, texture: Texture) -> Self {
        self.textures.push(texture);
        self
    }

    pub fn add_mesh(mut self, mesh: Mesh) -> Self {
        let index_start = self.vertices.len();
        self.vertices.append(&mut mesh.vertices.into_vec());
        self.indices.extend(mesh.indices.iter().map(|x| *x + index_start as u32));
        self.textures.append(
            &mut mesh.textures.map_or(Vec::new(), |x| x.into_vec())
        );
        self
    }

    pub fn extend_vertex(mut self, verts: Vec<Vertex>) -> Self {
        let mut verts = verts;
        self.vertices.append(&mut verts);
        self
    }

    pub fn extend_index(mut self, indices: Vec<u32>) -> Self {
        let mut indices = indices;
        self.indices.append(&mut indices);
        self
    }

    pub fn extend_texture(mut self, textures: Vec<Texture>) -> Self {
        let mut textures = textures;
        self.textures.append(&mut textures);
        self
    }

    pub fn extend_mesh(mut self, mesh: &Mesh) -> Self {
        let index_start = self.vertices.len();
        self.vertices.extend_from_slice(mesh.vertices());
        self.indices.extend(mesh.indices().iter().map(|x| *x + index_start as u32));
        self.textures.extend_from_slice(
            mesh.textures().as_ref().map_or(&[], |x| x.as_ref())
        );
        self
    }

    pub fn build(self) -> Mesh {
        let Self { vertices, indices, textures } = self;
        
        let textures = if textures.is_empty() {
            None
        } else {
            Some(textures.into_boxed_slice())
        };

        Mesh {
            vertices: vertices.into_boxed_slice(),
            indices: indices.into_boxed_slice(),
            textures: textures
        }
    }
}
