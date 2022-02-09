use std::marker::PhantomData;

use crate::prelude::*;
use anyhow::anyhow;

type RawMemAddr = hecs::Entity;

#[derive(Clone)]
pub struct MemAddr<T> {
    addr: RawMemAddr,
    typ: PhantomData<T>,
}
impl<T: Clone> Copy for MemAddr<T> {}

impl<T> std::fmt::Debug for MemAddr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemAddr")
            .field("addr", &self.addr)
            .field("typ", &self.typ)
            .finish()
    }
}

impl<T> MemAddr<T>
where
    T: Send + Sync + 'static,
{
    /// Type-erases the memory address. For internal use only.
    fn to_raw(&self) -> RawMemAddr {
        self.addr
    }

    /// Enriches a address with type information
    fn from_raw(addr: RawMemAddr) -> Self {
        Self {
            addr,
            typ: PhantomData,
        }
    }

    /// Enriches a address with type information. Returns error if the value is
    /// not of the right type or hasn't been stored.
    pub fn from_raw_checked(
        program: &PolyAsmProgram,
        addr: RawMemAddr,
        param_name: &str,
    ) -> Result<Self> {
        if program.memory.get::<T>(addr).is_ok() {
            Ok(Self {
                addr,
                typ: PhantomData,
            })
        } else {
            Err(anyhow!(
                "Could not make address. The value of param {:?} is not of the right type.",
                param_name,
            ))
        }
    }
}

/// The instructions of the PolyAsm language. Like a typed quadruple-based IR
/// language, where the memory addresses of input and output operands are
/// specified.
#[derive(Clone)]
pub enum PolyAsmInstruction {
    MakeCube {
        origin: MemAddr<Vec3>,
        size: MemAddr<Vec3>,
        out_mesh: MemAddr<HalfEdgeMesh>,
    },
    MakeQuad {
        center: MemAddr<Vec3>,
        normal: MemAddr<Vec3>,
        right: MemAddr<Vec3>,
        size: MemAddr<Vec3>,
        out_mesh: MemAddr<HalfEdgeMesh>,
    },
    ChamferVertices {
        vertices: MemAddr<Vec<u32>>,
        amount: MemAddr<f32>,
        in_mesh: MemAddr<HalfEdgeMesh>,
        out_mesh: MemAddr<HalfEdgeMesh>,
    },
    BevelEdges {
        edges: MemAddr<Vec<u32>>,
        amount: MemAddr<f32>,
        in_mesh: MemAddr<HalfEdgeMesh>,
        out_mesh: MemAddr<HalfEdgeMesh>,
    },
    ExtrudeFaces {
        faces: MemAddr<Vec<u32>>,
        amount: MemAddr<f32>,
        in_mesh: MemAddr<HalfEdgeMesh>,
        out_mesh: MemAddr<HalfEdgeMesh>,
    },
    MakeVector {
        x: MemAddr<f32>,
        y: MemAddr<f32>,
        z: MemAddr<f32>,
        out_vec: MemAddr<Vec3>,
    },
    VectorAdd {
        a: MemAddr<Vec3>,
        b: MemAddr<Vec3>,
        out_vec: MemAddr<Vec3>,
    },
    VectorSub {
        a: MemAddr<Vec3>,
        b: MemAddr<Vec3>,
        out_vec: MemAddr<Vec3>,
    },
    MergeMeshes {
        a: MemAddr<HalfEdgeMesh>,
        b: MemAddr<HalfEdgeMesh>,
        out_mesh: MemAddr<HalfEdgeMesh>,
    },
    LinearSubdivide {
        in_mesh: MemAddr<HalfEdgeMesh>,
        out_mesh: MemAddr<HalfEdgeMesh>,
    },
    ExportObj {
        in_mesh: MemAddr<HalfEdgeMesh>,
        export_path: MemAddr<std::path::PathBuf>,
    },
}

pub struct PolyAsmProgram {
    instructions: Vec<PolyAsmInstruction>,
    output_register: Option<MemAddr<HalfEdgeMesh>>,
    memory: hecs::World,
}

impl PolyAsmProgram {
    pub fn new() -> PolyAsmProgram {
        let world = hecs::World::new();
        Self {
            instructions: vec![],
            output_register: None,
            memory: world,
        }
    }

    pub fn mem_alloc_raw<T: Send + Sync + 'static>(&mut self, value: T) -> RawMemAddr {
        self.memory.spawn((value,))
    }

    /// Reserves the space for a value of type T in the memory and returns its
    /// address. No value is actually stored. The value is uninitialized, so
    /// fetching will fail if this address is used before a store.
    pub fn mem_reserve<T: Send + Sync + 'static>(&mut self) -> MemAddr<T> {
        MemAddr::from_raw(self.memory.reserve_entity())
    }

    pub fn mem_store<T: Send + Sync + 'static>(
        &mut self,
        addr: MemAddr<T>,
        value: T,
    ) -> Result<()> {
        self.memory
            .insert(addr.to_raw(), (value,))
            .map_err(|err| anyhow!("Error storing").context(err))
    }

    pub fn mem_retrieve<T: Send + Sync + 'static>(&mut self, addr: MemAddr<T>) -> Result<T> {
        self.memory
            .remove_one::<T>(addr.to_raw())
            .map_err(|_err| anyhow!("Error retrieving from mem"))
    }

    pub fn mem_fetch<T: Send + Sync + 'static + Clone>(&self, addr: MemAddr<T>) -> Result<T> {
        self.memory
            .get::<T>(addr.to_raw())
            .map_err(|_err| anyhow!("Memory fetch error for address {:?}", addr))
            .map(|x| (*x).clone())
    }

    pub fn mem_fetch_ref<T: Send + Sync + 'static>(
        &self,
        addr: MemAddr<T>,
    ) -> Result<hecs::Ref<T>> {
        self.memory
            .get::<T>(addr.to_raw())
            .map_err(|_err| anyhow!("Memory fetch error for address {:?}", addr))
    }

    pub fn add_operation(&mut self, op: PolyAsmInstruction) {
        self.instructions.push(op);
    }

    pub fn execute_instruction(&mut self, instr: PolyAsmInstruction) -> Result<()> {
        match &instr {
            PolyAsmInstruction::MakeCube {
                origin: center,
                size,
                out_mesh,
            } => {
                let center = self.mem_fetch(*center)?;
                let size = self.mem_fetch(*size)?;
                self.mem_store(*out_mesh, halfedge::primitives::Box::build(center, size))?;
                self.output_register = Some(*out_mesh);
            }
            PolyAsmInstruction::MakeQuad {
                center,
                normal,
                right,
                size,
                out_mesh,
            } => {
                let center = self.mem_fetch(*center)?;
                let normal = self.mem_fetch(*normal)?;
                let right = self.mem_fetch(*right)?;
                let size = self.mem_fetch(*size)?;
                self.mem_store(
                    *out_mesh,
                    halfedge::primitives::Quad::build(center, normal, right, size.truncate()),
                )?;
                self.output_register = Some(*out_mesh);
            }
            PolyAsmInstruction::ChamferVertices {
                vertices,
                amount,
                in_mesh,
                out_mesh,
            } => {
                let vertices = self.mem_fetch(*vertices)?;
                let amount = self.mem_fetch(*amount)?;
                let mut result = (*self.mem_fetch_ref(*in_mesh)?).clone();

                result.clear_debug();
                let vs = result.iter_vertices().map(|x| x.0).collect::<Vec<_>>();
                for vertex in vertices {
                    let v_id = vs
                        .get(vertex as usize)
                        .cloned()
                        .ok_or_else(|| anyhow!("Invalid index: {}", vertex))?;

                    halfedge::edit_ops::chamfer_vertex(&mut result, v_id, amount)?;
                }
                self.mem_store(*out_mesh, result)?;
                self.output_register = Some(*out_mesh);
            }
            PolyAsmInstruction::BevelEdges {
                edges,
                amount,
                in_mesh,
                out_mesh,
            } => {
                let edges = self.mem_fetch(*edges)?;
                let amount = self.mem_fetch(*amount)?;
                let mut result = (*self.mem_fetch_ref(*in_mesh)?).clone();

                result.clear_debug();
                let hs = result.iter_halfedges().map(|x| x.0).collect::<Vec<_>>();
                let edges_to_bevel = edges
                    .iter()
                    .map(|idx| {
                        hs.get(*idx as usize)
                            .cloned()
                            .ok_or_else(|| anyhow!("Invalid index: {}", idx))
                    })
                    .collect::<Result<Vec<_>>>()?;
                halfedge::edit_ops::bevel_edges(&mut result, &edges_to_bevel, amount)?;

                self.mem_store(*out_mesh, result)?;
                self.output_register = Some(*out_mesh);
            }
            PolyAsmInstruction::ExtrudeFaces {
                faces,
                amount,
                in_mesh,
                out_mesh,
            } => {
                let faces = self.mem_fetch(*faces)?;
                let amount = self.mem_fetch(*amount)?;
                let mut result = (*self.mem_fetch_ref(*in_mesh)?).clone();

                result.clear_debug();
                let fs = result.iter_faces().map(|x| x.0).collect::<Vec<_>>();
                let faces_to_extrude = faces
                    .iter()
                    .map(|idx| {
                        fs.get(*idx as usize)
                            .cloned()
                            .ok_or_else(|| anyhow!("Invalid index: {}", idx))
                    })
                    .collect::<Result<Vec<_>>>()?;
                halfedge::edit_ops::extrude_faces(&mut result, &faces_to_extrude, amount)?;

                self.mem_store(*out_mesh, result)?;
                self.output_register = Some(*out_mesh);
            }
            PolyAsmInstruction::MakeVector { x, y, z, out_vec } => {
                let x = self.mem_fetch(*x)?;
                let y = self.mem_fetch(*y)?;
                let z = self.mem_fetch(*z)?;
                self.mem_store(*out_vec, Vec3::new(x, y, z))?;
            }
            PolyAsmInstruction::VectorAdd { a, b, out_vec } => {
                let a = self.mem_fetch(*a)?;
                let b = self.mem_fetch(*b)?;
                self.mem_store(*out_vec, a + b)?;
            }
            PolyAsmInstruction::VectorSub { a, b, out_vec } => {
                let a = self.mem_fetch(*a)?;
                let b = self.mem_fetch(*b)?;
                self.mem_store(*out_vec, a - b)?;
            }
            PolyAsmInstruction::MergeMeshes { a, b, out_mesh } => {
                let result = {
                    // Extra scope required to not keep refs alive
                    let mesh_a = &*self.mem_fetch_ref(*a)?;
                    let mesh_b = &*self.mem_fetch_ref(*b)?;

                    let mut result = mesh_a.clone();
                    result.clear_debug();
                    result.merge_with(mesh_b);
                    result
                };
                self.mem_store(*out_mesh, result)?;
                self.output_register = Some(*out_mesh);
            }
            PolyAsmInstruction::ExportObj {
                in_mesh,
                export_path,
            } => {
                let mesh = &*self.mem_fetch_ref(*in_mesh)?;
                let export_path = self.mem_fetch(*export_path)?;
                mesh.to_wavefront_obj(export_path)?;
            }
            PolyAsmInstruction::LinearSubdivide { in_mesh, out_mesh } => {
                let new_mesh = halfedge::compact_mesh::CompactMesh::from_halfedge(
                    &*self.mem_fetch_ref(*in_mesh)?,
                )?;

                let subdivided = new_mesh.subdivide_halfedge_refinement().to_halfedge();

                self.mem_store(*out_mesh, subdivided)?;
                self.output_register = Some(*out_mesh);
            }
        }
        Ok(())
    }

    pub fn execute(mut self) -> Result<HalfEdgeMesh> {
        let instructions = self.instructions.clone();
        for instruction in instructions.into_iter() {
            self.execute_instruction(instruction)?;
        }

        if let Some(output_register) = self.output_register {
            self.mem_retrieve(output_register)
        } else {
            Err(anyhow!("No operations produced output"))
        }
    }
}

impl Default for PolyAsmProgram {
    fn default() -> Self {
        Self::new()
    }
}
