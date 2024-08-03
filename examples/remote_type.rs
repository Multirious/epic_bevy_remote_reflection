/// Pretend this is a crate
#[allow(unused)]
mod cool_crate {
    pub mod cool_module {
        /// Pretend this is a remote type
        #[derive(Debug)]
        pub struct CoolType {
            pub id: usize,
            pub things: Vec<String>,
        }
    }
}

// ============================================================================

use bevy_reflect::{Reflect, ReflectRef};
use cool_crate::cool_module::CoolType;
use epic_bevy_remote_reflection::{ReflectViaExt, RemoteReflect, RemoteReflectList};
use std::any::TypeId;

// Must structured exactly like the remote type
// Changing type_path and type_name is not a requirement but it's a good idea
// to include them as well.
#[derive(Debug, Reflect)]
#[type_path = "cool_crate::cool_module"]
#[type_name = "CoolType"]
struct RemoteCoolType {
    id: usize,
    things: Vec<String>,
}

unsafe impl RemoteReflect for RemoteCoolType {
    type Item = CoolType;
}

// You can use this API so you only need one namespace for all your
// remote reflection need!
// Let's go do UB ergonomically!
struct MyReflect;
impl RemoteReflectList<CoolType> for MyReflect {
    type RemoteReflector = RemoteCoolType;
}

fn main() {
    unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    let cool_value = CoolType {
        id: 69,
        things: vec!["Book".to_string(), "Table".to_string()],
    };
    let reflected = cool_value.as_reflect_via::<MyReflect>();
    assert_eq!(reflected.type_id(), TypeId::of::<CoolType>());
    assert_eq!(reflected.as_any().type_id(), TypeId::of::<CoolType>());
    assert!(reflected.as_any().downcast_ref::<CoolType>().is_some());
    println!("{:?}", reflected.as_any().downcast_ref::<CoolType>());
    println!("{:?}", reflected);

    let boxed = Box::new(cool_value).into_reflect_via::<MyReflect>();
    let ReflectRef::Struct(struct_ref) = boxed.reflect_ref() else {
        unreachable!()
    };
    assert_eq!(
        struct_ref.field("id").unwrap().downcast_ref::<usize>(),
        Some(&69)
    );
    let a = struct_ref
        .field("things")
        .unwrap()
        .downcast_ref::<Vec<String>>()
        .unwrap();
    let b = vec!["Book".to_string(), "Table".to_string()];
    assert_eq!(a, &b);
}
