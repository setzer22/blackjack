use blackjack_engine::gizmos::{BlackjackGizmo, TransformGizmo};
use epaint::{text::cursor, Pos2, Vec2};
use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};

use crate::{renderer::{id_picking_routine::PickableId, ViewportCamera}, viewport_3d::viewport_math::{Ray, Plane}};


pub fn gizmo_update(
    viewport_size: Vec2,
    cursor_position: Pos2,
    drag_delta: Option<Vec2>,
    camera: &ViewportCamera,
    subgizmo_under_cursor: Option<PickableId>,
) {
    let ray = Ray::from_screenspace(
        camera,
        cursor_position.to_vec2(),
        viewport_size
    );
    
    let point_in_xz = ray.intersect_plane(&Plane {
        point: Vec3::new(0.0, 0.0, 0.0),
        normal: Vec3::new(0.0, 1.0, 0.0),
    });
    
    let point_in_x_line = ray.closest_point_to_line(&Ray {
        origin: Vec3::new(0.0, 0.0, 0.0),
        direction: Vec3::new(1.0, 0.0, 0.0),
    });
    
    dbg!(point_in_xz);
    dbg!(point_in_x_line);


    /*match gizmo {
        BlackjackGizmo::Transform(transform) => match transform.gizmo_mode {
            blackjack_engine::gizmos::TransformGizmoMode::Translate => {
                if let Some(gizmo_part) = subgizmo_under_cursor.and_then(|g| g.get_subgizmo_index())
                {
                    match gizmo_part {
                        // X axis
                        0 => {

                        }
                        // Y axis
                        1 => {}
                        // Z axis
                        2 => {}
                        // XY plane
                        3 => {}
                        // XZ plane
                        4 => {}
                        // YZ plane
                        5 => {}
                        _ => {
                            unreachable!("Invalid gizmo part index for translate gizmo: {}", gizmo_part);
                        }
                    }
                }
            }
            blackjack_engine::gizmos::TransformGizmoMode::Rotate => {
                log::warn!("Ignored rotate gizmo");
            }
            blackjack_engine::gizmos::TransformGizmoMode::Scale => {
                log::warn!("Ignored scale gizmo");
            }
        },
        BlackjackGizmo::None => todo!(),
    }*/
}
