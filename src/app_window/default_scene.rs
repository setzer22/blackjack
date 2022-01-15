use crate::{mesh::debug_viz::DebugMeshes, mesh::halfedge, prelude::*};

fn load_obj(path: &str) -> halfedge::HalfEdgeMesh {
    /*
    let model = wavefront::Obj::from_file(path).unwrap();
    let mut polygons = vec![];
    for polygon in model.polygons() {
        let p: smallvec::SmallVec<[usize; 4]> =
            polygon.vertices().map(|v| v.position_index()).collect();
        polygons.push(p);
    }
    let positions: Vec<glam::Vec3> = model
        .positions()
        .iter()
        .map(|[x, y, z]| glam::Vec3::new(*x, *y, *z))
        .collect();
    halfedge::HalfEdgeMesh::build_from_polygons(&positions, &polygons).unwrap()
    */
    todo!()
}

fn debug_vertex_ids(mesh: &mut halfedge::HalfEdgeMesh) {
    let vs = mesh.iter_vertices().map(|x| x.0).collect::<Vec<_>>();
    for v in vs {
        mesh.add_debug_vertex(
            v,
            DebugMark::new(v.idx().to_string().as_str(), egui::Color32::RED),
        );
    }
}

fn debug_halfedge_ids(mesh: &mut halfedge::HalfEdgeMesh) {
    let es = mesh.iter_halfedges().map(|x| x.0).collect::<Vec<_>>();
    for h in es {
        mesh.add_debug_halfedge(h, DebugMark::blue(h.idx().to_string().as_str()));
    }
}

fn bevel_edge_test_case_1() -> halfedge::HalfEdgeMesh {
    let mut mesh = load_obj("assets/bevel_edge_test_case_1.obj");

    let v0 = mesh.iter_vertices().nth(0).unwrap().0;
    let v1 = mesh.iter_vertices().nth(1).unwrap().0;
    let v2 = mesh.iter_vertices().nth(2).unwrap().0;
    let v3 = mesh.iter_vertices().nth(3).unwrap().0;
    let v4 = mesh.iter_vertices().nth(4).unwrap().0;
    let v5 = mesh.iter_vertices().nth(5).unwrap().0;
    let v6 = mesh.iter_vertices().nth(6).unwrap().0;

    //halfedge::edit_ops::split_vertex(&mut mesh, v1, v0, v4).expect("Could split vertex");

    let v7 = halfedge::edit_ops::split_vertex(
        &mut mesh,
        v5,
        v6,
        v4,
        Vec3::Y * 0.5 + Vec3::Z * 0.5,
        false,
    )
    .expect("Could split vertex");
    let v8 = halfedge::edit_ops::split_vertex(
        &mut mesh,
        v4,
        v5,
        v3,
        Vec3::X * 0.5 + Vec3::Z * 0.5,
        false,
    )
    .expect("Could split vertex");
    let v9 = halfedge::edit_ops::split_vertex(
        &mut mesh,
        v3,
        v4,
        v2,
        Vec3::Y * -0.5 + Vec3::Z * -0.1,
        false,
    )
    .expect("Could split vertex");

    let h_5_8 = mesh.at_vertex(v5).halfedge_to(v8).end();
    halfedge::edit_ops::dissolve_edge(&mut mesh, h_5_8).expect("Dissolve");

    let h_4_9 = mesh.at_vertex(v4).halfedge_to(v9).end();
    halfedge::edit_ops::dissolve_edge(&mut mesh, h_4_9).expect("Dissolve");

    mesh
}

fn bevel_edge_test_case_2() -> halfedge::HalfEdgeMesh {
    let mut mesh = load_obj("assets/bevel_edge_test_case_2.obj");
    debug_vertex_ids(&mut mesh);

    let vs = mesh.iter_vertices().map(|p| p.0).collect::<Vec<_>>();

    // Split a bunch of edges (pseudo-bevel)
    let h = mesh.at_vertex(vs[9]).halfedge_to(vs[16]).end();
    halfedge::edit_ops::split_edge(&mut mesh, h, Vec3::Z * 0.35, false).unwrap();

    let h = mesh.at_vertex(vs[10]).halfedge_to(vs[9]).end();
    halfedge::edit_ops::split_edge(&mut mesh, h, -Vec3::X * 0.35, false).unwrap();

    let h = mesh.at_vertex(vs[19]).halfedge_to(vs[10]).end();
    halfedge::edit_ops::split_edge(&mut mesh, h, -Vec3::Z * 0.35, false).unwrap();

    let h = mesh.at_vertex(vs[16]).halfedge_to(vs[19]).end();
    halfedge::edit_ops::split_edge(&mut mesh, h, Vec3::X * 0.35, false).unwrap();

    // Divide a random edge
    let h = mesh.at_vertex(vs[56]).halfedge_to(vs[57]).end();
    halfedge::edit_ops::divide_edge(&mut mesh, h, 0.5).unwrap();

    let h = mesh.at_vertex(vs[50]).halfedge_to(vs[59]).end();
    halfedge::edit_ops::divide_edge(&mut mesh, h, 0.5).unwrap();
    let h = mesh.at_vertex(vs[50]).halfedge_to(vs[5]).end();
    halfedge::edit_ops::divide_edge(&mut mesh, h, 0.5).unwrap();
    let h = mesh.at_vertex(vs[5]).halfedge_to(vs[52]).end();
    halfedge::edit_ops::divide_edge(&mut mesh, h, 0.5).unwrap();
    let h = mesh.at_vertex(vs[59]).halfedge_to(vs[52]).end();
    halfedge::edit_ops::divide_edge(&mut mesh, h, 0.5).unwrap();

    halfedge::edit_ops::cut_face(&mut mesh, vs[52], vs[50]).unwrap();

    halfedge::edit_ops::dissolve_vertex(&mut mesh, vs[60]).unwrap();

    let vs = mesh
        .iter_vertices()
        .map(|(v_id, _)| (v_id.into(), v_id))
        .collect::<HashMap<usize, VertexId>>();

    debug_vertex_ids(&mut mesh);

    mesh.add_debug_vertex(vs[&82], DebugMark::blue("82"));
    halfedge::edit_ops::dissolve_vertex(&mut mesh, vs[&82]).unwrap();
    halfedge::edit_ops::dissolve_vertex(&mut mesh, vs[&87]).unwrap();
    halfedge::edit_ops::dissolve_vertex(&mut mesh, vs[&85]).unwrap();

    halfedge::edit_ops::chamfer_vertex(&mut mesh, vs[&29], 0.5).unwrap();

    let h = mesh.at_vertex(vs[&20]).halfedge_to(vs[&21]).end();
    halfedge::edit_ops::duplicate_edge(&mut mesh, h).unwrap();
    let (f, _) = halfedge::edit_ops::chamfer_vertex(&mut mesh, vs[&21], 0.5).unwrap();

    for (i, v) in mesh.at_face(f).vertices().unwrap().iter().enumerate() {
        mesh.add_debug_vertex(*v, DebugMark::blue(&format!("{:?}", i)))
    }

    let vnew = mesh.at_face(f).vertices().unwrap()[1];
    mesh.update_vertex_position(vnew, |vold| vold + Vec3::new(0.0, 0.0, -0.2));

    mesh
}

fn bevel_edge_test_case_3() -> halfedge::HalfEdgeMesh {
    let mut mesh = halfedge::primitives::Box::build(Vec3::ZERO, Vec3::ONE * 3.0);

    let edges: Vec<_> = mesh.iter_halfedges().map(|x| x.0).collect();
    let to_bevel = &[edges[6], edges[13], edges[1], edges[2], edges[3]];
    //let to_bevel = &[edges[6]];
    halfedge::edit_ops::bevel_edges(&mut mesh, to_bevel, 0.3).unwrap();

    debug_vertex_ids(&mut mesh);

    mesh
}

fn test_divide_edge() -> halfedge::HalfEdgeMesh {
    let mut mesh = halfedge::primitives::Quad::build(Vec3::ZERO, Vec3::Y, Vec3::X, Vec2::ONE);
    let hs: Vec<HalfEdgeId> = mesh.iter_halfedges().map(|x| x.0).collect();
    halfedge::edit_ops::divide_edge(&mut mesh, hs[0], 0.5).unwrap();

    debug_vertex_ids(&mut mesh);
    debug_halfedge_ids(&mut mesh);

    for (h, halfedge) in mesh.iter_halfedges() {
        println!("Halfedge: {:?}", h.idx());
        dbg!(halfedge);
    }

    for (v, vertex) in mesh.iter_vertices() {
        println!("Vertex: {:?}", v.idx());
        dbg!(vertex);
    }

    for (f, face) in mesh.iter_faces() {
        println!("Face: {:?}", f.idx());
        dbg!(face);
    }

    mesh
}

fn make_halfedge_mesh() -> halfedge::HalfEdgeMesh {
    bevel_edge_test_case_3()
}

pub fn build_mesh(mesh: &halfedge::HalfEdgeMesh) -> r3::Mesh {
    let (positions, indices) = mesh.generate_buffers();
    r3::MeshBuilder::new(positions)
        .with_indices(indices)
        .build()
        .unwrap()
}

pub fn add_default_scene(render_ctx: &mut RenderContext, debug_meshes: &mut DebugMeshes) {
    /*
    let hm = make_halfedge_mesh();
    render_ctx.add_mesh_as_object(build_mesh(&hm));

    let objects = debug_viz::add_halfedge_debug(&render_ctx.renderer, debug_meshes, &hm);
    for obj in objects {
        Box::leak(Box::new(obj));
    }
    */

    let view_location = glam::Vec3::new(3.0, 3.0, -5.0);
    let view = glam::Mat4::from_euler(glam::EulerRot::XYZ, -0.55, 0.5, 0.0);
    let view = view * glam::Mat4::from_translation(-view_location);

    render_ctx.set_camera(view);

    render_ctx.add_light(r3::DirectionalLight {
        color: glam::Vec3::ONE,
        intensity: 10.0,
        // Direction will be normalized
        direction: glam::Vec3::new(-1.0, -4.0, 2.0),
        distance: 400.0,
    });

    //hm
}
