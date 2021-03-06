use cgmath::{ Point3, Vector3 };
use gekraftet_core::world::{ self, Chunk, Section, SectionPos };
use gekraftet_core::utils::PartialArray;
use crate::mesh::{ Face, Mesh, MeshBuilder };
use super::{ Mesher, BLOCK_LENGTH };

pub struct GreedyCubeMesher<'a> {
    chunk: &'a Chunk,
}

#[derive(Clone, Debug, Default)]
struct GroupedBlock {
    // This bitfield is filled with the following information:
    // - block extent: (x, y, z) = 12 bits (4 bits * 3) (see note 1)
    // - block type:   (indexed) = 12 bits (see note 2)
    // - block faces:            =  6 bits (one for each face)
    // - group info:         (g) =  1 bits (g = is in group)
    // ----------------------------------------------------------------
    //                     TOTAL = 31 bits (4 bytes needed)
    // 
    // NOTE #1: Since it is impossible to have 0 extent, 0 represents 16.
    //          This allows us to save one bit of memory. 
    // NOTE #2: There are 4096 *different* blocks at best in each section,
    //          thus we store an index that points to the actual block data.
    //          This way we need 12 bits only - while saving the whole block
    //          data can take up to 8 bytes of data per group!
    bitfield: u32,
}

impl GroupedBlock {
    #[inline] // since this is used here only
    fn new(block: u16) -> Self {
        // default extent: (1, 1, 1)
        let extent = 0b100010001;
        let faces = 0b111111 << 24;

        Self {
            bitfield: extent | ((block as u32) << 12) | faces
        }
    }

    fn block_id(&self) -> usize {
        ((self.bitfield >> 12) & 0xFFF) as usize
    }

    fn extent(&self) -> Vector3<i32> {
        let x = (self.bitfield >> 8) & 0xF;
        let y = (self.bitfield >> 4) & 0xF;
        let z = (self.bitfield >> 0) & 0xF;
        
        let x = if x == 0 { 16 } else { x };
        let y = if y == 0 { 16 } else { y };
        let z = if z == 0 { 16 } else { z };

        Vector3::<i32>::new(x as i32, y as i32, z as i32)
    }

    fn is_in_group(&self) -> bool {
        (self.bitfield >> 30) & 1 == 1
    }

    fn faces(&self) -> Face {
        Face::from_bitfield((self.bitfield >> 24) as u8 & 0b111111)
    }

    fn extend_to(&mut self, x: usize, y: usize, z: usize) {
        let bits = (x & 0xF) << 8 | (y & 0xF) << 4 | (z & 0xF) << 0;
        let mask = !0xFFF;
        self.bitfield &= mask;
        self.bitfield |= bits as u32;
    }

    fn toggle_group(&mut self) {
        self.bitfield ^= 1 << 30;
    }

    fn set_faces(&mut self, face: Face) {
        let faces = (face.into_bitfield() as u32) << 24;
        let mask = !(0b111111 << 24);
        self.bitfield &= mask;
        self.bitfield |= faces;
    }
}

impl<'a> GreedyCubeMesher<'a> {
    fn intrasection_cull(
        &self,
        section_pos: SectionPos,
        section: &Section,
    ) -> Mesh 
    {
        let block_pos = *section_pos * 16;

        let mut blocks = Vec::with_capacity(16);
        let mut groups: [GroupedBlock; 4096] = {
            let mut g = PartialArray::<GroupedBlock, 4096>::new();

            let range = 
                (0..16)
                    .flat_map(move |x| (0..16)
                        .map(move |z| (x, z)));

            // initialization and a marking pass along y-axis
            for (x, z) in range {
                for y in 0..16 {
                    let block_id = blocks.iter().enumerate().rev().find(|b| {
                        b.1 == &&section[x][z][y]
                    });

                    let block_id = match block_id {
                        Some((i, _)) => i as u16,
                        None => {
                            blocks.push(&section[x][z][y]);
                            (blocks.len() - 1) as u16
                        },
                    };

                    let mut group = GroupedBlock::new(block_id);

                    if y > 0 {
                        let b = g.get_mut(x * 256 + z * 16 + y - 1).unwrap();
                        
                        let can_disable_face =
                            blocks[b.block_id()].id != 0 && 
                            blocks[group.block_id()].id != 0;

                        let mut face1 = group.faces();
                        let mut face2 = b.faces();

                        if b.block_id() == group.block_id() {
                            group.extend_to(1, 1 + b.extent().y as usize, 1);
                            b.toggle_group();
                        } else if can_disable_face {
                            face1.disable(Face::BOTTOM);
                            face2.disable(Face::TOP);
                            group.set_faces(face1);
                            b.set_faces(face2);
                        }
                    };

                    g.push(group).unwrap();
                }
            };

            g.into_full_array().unwrap()
        };

        // marking along z-axis
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..16 {
                    if z == 0 { continue };
        
                    let idx = x * 256 + z * 16 + y;
                    let idx2 = idx - 16;
        
                    if groups[idx].is_in_group() {
                        continue
                    };

                    let can_disable_face =
                        blocks[groups[idx].block_id()].id != 0 && 
                        blocks[groups[idx2].block_id()].id != 0 &&
                        groups[idx2].extent().y >= groups[idx].extent().y;

                    if groups[idx2].is_in_group() {
                        if can_disable_face {
                            let mut face = groups[idx].faces();
                            face.disable(Face::BACK);
                            groups[idx].set_faces(face);
                        }
                        continue
                    };
                    
                    if groups[idx].extent().y == groups[idx2].extent().y {
                        let mut face1 = groups[idx].faces();
                        let mut face2 = groups[idx2].faces();

                        if groups[idx].block_id() == groups[idx2].block_id() {
                            groups[idx2].toggle_group();
                            
                            let orig_ext = groups[idx].extent();
                            groups[idx].extend_to(
                                orig_ext.x as usize,
                                orig_ext.y as usize,
                                (orig_ext.z + groups[idx2].extent().z) as usize,
                            );
                        } else if can_disable_face {
                            face1.disable(Face::BACK);
                            face2.disable(Face::FRONT);
                            groups[idx].set_faces(face1);
                            groups[idx2].set_faces(face2);
                        }
                    }
                }
            }
        }

        // marking along x-axis
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..16 {
                    if x == 0 { continue };
        
                    let idx = x * 256 + z * 16 + y;
                    let idx2 = idx - 256;
        
                    if groups[idx].is_in_group() {
                        continue
                    };

                    let can_disable_face =
                        blocks[groups[idx].block_id()].id != 0 && 
                        blocks[groups[idx2].block_id()].id != 0 &&
                        groups[idx2].extent().y >= groups[idx].extent().y &&
                        groups[idx2].extent().z >= groups[idx].extent().z;

                    if groups[idx2].is_in_group() {
                        if can_disable_face {
                            let mut face = groups[idx].faces();
                            face.disable(Face::LEFT);
                            groups[idx].set_faces(face);
                        }
                        continue
                    };
                    
                    if groups[idx].extent().y == groups[idx2].extent().y &&
                       groups[idx].extent().z == groups[idx2].extent().z 
                    {
                        let mut face1 = groups[idx].faces();
                        let mut face2 = groups[idx2].faces();

                        if groups[idx].block_id() == groups[idx2].block_id() {
                            groups[idx2].toggle_group();
                            
                            let orig_ext = groups[idx].extent();
                            
                            groups[idx].extend_to(
                                (orig_ext.x + groups[idx2].extent().x) as usize,
                                orig_ext.y as usize,
                                orig_ext.z as usize,
                            );
                        } else if can_disable_face {
                            face1.disable(Face::LEFT);
                            face2.disable(Face::RIGHT);
                            groups[idx].set_faces(face1);
                            groups[idx2].set_faces(face2);
                        }
                    }
                }
            }
        }

        let mut mb = MeshBuilder::new();
        
        for (pos, grp) in groups.iter().enumerate() {
            if grp.is_in_group() { 
                continue 
            };

            if blocks[grp.block_id()].id == 0 {
                continue
            };

            let x = ((pos >> 8) & 0xF) as i32;
            let z = ((pos >> 4) & 0xF) as i32;
            let y = ((pos >> 0) & 0xF) as i32;
            let extent = grp.extent().cast::<f32>().unwrap();
            let origin = Point3::<i32>::new(x, y, z)
                + block_pos.to_homogeneous().truncate()
                - grp.extent();

            let mesh = MeshBuilder::create_cuboid(
                extent * BLOCK_LENGTH, 
                (origin.cast::<f32>().unwrap() + 0.5 * extent) * BLOCK_LENGTH,
                grp.faces()
            );
            
            mb = mb.add_mesh(mesh);
        }

        mb.build()
    }
}

impl<'a> Mesher<'a> for GreedyCubeMesher<'a> {
    fn from_chunk(chunk: &'a Chunk) -> Self {
        assert!(
            world::SECTION_LENGTH_X <= 16
            && world::SECTION_LENGTH_Y <= 16
            && world::SECTION_LENGTH_Z <= 16,
            "GreedyCubeMesher is designed for sections that are 16x16x16 blocks"
        );

        Self {
            chunk
        }
    }

    fn generate_mesh(&self) -> Mesh {
        let mut meshes = MeshBuilder::new();
        for (i, sect) in self.chunk.sections().iter().enumerate() {
            let sect_pos = SectionPos::new(
                self.chunk.position().x,
                self.chunk.position().y + i as i32,
                self.chunk.position().z,
            );
            meshes = meshes.add_mesh(self.intrasection_cull(sect_pos, sect));
        };
        meshes.build()
    }
}
