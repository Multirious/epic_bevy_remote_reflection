use std::any::{Any, TypeId};
use std::mem::{transmute, transmute_copy, ManuallyDrop};

use bevy_reflect::{
    serde::Serializable, ApplyError, Reflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef,
    TypeInfo,
};

const fn new_vtable<This: RemoteReflect>() -> &'static ReflectVtable<This> {
    &const {
        let new_any_vtable = const { &AnyVtable::<This::Item>::new() };
        let new_common_vtable = const { &CommonVtable::<This::Item>::new() };
        unsafe {
            let mut new = ReflectVtable::<This>::new(
                new_any_vtable as *const _ as *const _,
                new_common_vtable as *const _ as *const _,
                new_common_vtable as *const _ as *const _,
            );

            new.type_id = |_| TypeId::of::<This::Item>();
            new.into_any = |this: Box<This>| -> Box<dyn Any> {
                transmute::<_, Box<This::Item>>(ManuallyDrop::new(this))
            };
            new.as_any = |this: *const This| -> *const dyn Any {
                let this: &dyn Any = transmute::<_, &This::Item>(&*this);
                this as _
            };
            new.as_any_mut = |this: *mut This| -> *mut dyn Any {
                let this: &mut dyn Any = transmute::<_, &mut This::Item>(&mut *this);
                this as _
            };
            new.into_reflect = |this: Box<This>| -> Box<dyn Reflect> {
                let this: Box<This> = transmute(ManuallyDrop::new(this));
                let mut this: Box<dyn Reflect> = this;
                let this_raw = &mut this as *mut _ as *mut TraitObject<ReflectVtable<This>>;
                (*this_raw).vtable = new_vtable::<This>();
                this
            };
            new.as_reflect = |this: *const This| -> *const dyn Reflect {
                let mut this: &dyn Reflect = transmute::<_, &This>(&*this);
                let this_raw = &mut this as *mut _ as *mut TraitObject<ReflectVtable<This>>;
                (*this_raw).vtable = new_vtable::<This>();
                this
            };
            new.as_reflect_mut = |this: *mut This| -> *mut dyn Reflect {
                let mut this: &mut dyn Reflect = transmute::<_, &mut This>(&mut *this);
                let this_raw = &mut this as *mut _ as *mut TraitObject<ReflectVtable<This>>;
                (*this_raw).vtable = new_vtable::<This>();
                this
            };
            new
        }
    }
}

/// # Safety
/// This trait must only implement on a remote type wrapper that is structured
/// exactly like the remote type.
pub unsafe trait RemoteReflect: Reflect + Sized {
    /// The remote type
    type Item: Send + Sync + 'static + Sized;

    fn remote_into_reflect(item: Box<Self::Item>) -> Box<dyn Reflect> {
        // Swapping the virtual table inside the trait object to make the remote
        // type wrapper acts exactly like the remote type.
        unsafe {
            let mut v: Box<dyn Reflect> = transmute_copy::<_, Box<Self>>(&ManuallyDrop::new(item));
            let dyn_reflect = &mut v as *mut _ as *mut TraitObject<ReflectVtable<Self>>;
            (*dyn_reflect).vtable = new_vtable::<Self>();
            v
        }
    }

    fn remote_as_reflect(item: &Self::Item) -> &dyn Reflect {
        unsafe {
            let mut v: &dyn Reflect = transmute_copy::<_, &Self>(&item);
            let dyn_reflect = &mut v as *mut _ as *mut TraitObject<ReflectVtable<Self>>;
            (*dyn_reflect).vtable = new_vtable::<Self>();
            v
        }
    }

    fn remote_as_reflect_mut(item: &mut Self::Item) -> &mut dyn Reflect {
        unsafe {
            let mut v: &mut dyn Reflect = transmute_copy::<_, &mut Self>(&item);
            let dyn_reflect = &mut v as *mut _ as *mut TraitObject<ReflectVtable<Self>>;
            (*dyn_reflect).vtable = new_vtable::<Self>();
            v
        }
    }
}

#[repr(C)]
#[derive(Clone)]
struct TraitObject<V: Sized> {
    data: *const (),
    vtable: *const V,
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(clippy::type_complexity)]
struct ReflectVtable<This: Sized> {
    drop_in_place: unsafe fn(*mut This),
    size: usize,
    align: usize,
    reflect_type_path: fn(*const This) -> *const str,
    reflect_short_type_path: fn(*const This) -> *const str,
    reflect_type_ident: fn(*const This) -> *const str,
    reflect_crate_name: fn(*const This) -> Option<*const str>,
    reflect_module_path: fn(*const This) -> Option<*const str>,
    type_id: fn(*const This) -> TypeId,
    trait_any_vtable: *const AnyVtable<This>,
    trait_send_vtable: *const CommonVtable<This>,
    trait_sync_vtable: *const CommonVtable<This>,
    get_represented_type_info: fn(*const This) -> Option<&'static TypeInfo>,
    into_any: fn(Box<This>) -> Box<dyn Any>,
    as_any: fn(*const This) -> *const dyn Any,
    as_any_mut: fn(*mut This) -> *mut dyn Any,
    into_reflect: fn(Box<This>) -> Box<dyn Reflect>,
    as_reflect: fn(*const This) -> *const dyn Reflect,
    as_reflect_mut: fn(*mut This) -> *mut dyn Reflect,
    apply: fn(*mut This, *const dyn Reflect),
    try_apply: fn(*mut This, *const dyn Reflect) -> Result<(), ApplyError>,
    set: fn(*mut This, Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>,
    reflect_kind: fn(*const This) -> ReflectKind,
    reflect_ref: fn(*const This) -> ReflectRef<'static>,
    reflect_mut: fn(*mut This) -> ReflectMut<'static>,
    reflect_owned: fn(Box<This>) -> ReflectOwned,
    clone_value: fn(*const This) -> Box<dyn Reflect>,
    reflect_hash: fn(*const This) -> Option<u64>,
    reflect_partial_eq: fn(*const This, &dyn Reflect) -> Option<bool>,
    debug: fn(*const This, *mut std::fmt::Formatter<'_>) -> std::fmt::Result,
    serializable: fn(*const This) -> Option<Serializable<'static>>,
    is_dynamic: fn(*const This) -> bool,
}

impl<This: Sized + Reflect> ReflectVtable<This> {
    #[allow(clippy::missing_transmute_annotations)]
    const fn new(
        trait_any_vtable: *const AnyVtable<This>,
        trait_send_vtable: *const CommonVtable<This>,
        trait_sync_vtable: *const CommonVtable<This>,
    ) -> ReflectVtable<This> {
        unsafe {
            ReflectVtable {
                drop_in_place: std::ptr::drop_in_place::<This>,
                size: std::mem::size_of::<This>(),
                align: std::mem::align_of::<This>(),
                reflect_type_path: transmute(This::reflect_type_path as *const usize),
                reflect_short_type_path: transmute(This::reflect_short_type_path as *const usize),
                reflect_type_ident: transmute(This::reflect_type_ident as *const usize),
                reflect_crate_name: transmute(This::reflect_crate_name as *const usize),
                reflect_module_path: transmute(This::reflect_module_path as *const usize),
                type_id: transmute(This::type_id as *const usize),
                trait_any_vtable,
                trait_send_vtable,
                trait_sync_vtable,
                get_represented_type_info: transmute(
                    This::get_represented_type_info as *const usize,
                ),
                into_any: transmute(This::into_any as *const usize),
                as_any: transmute(This::as_any as *const usize),
                as_any_mut: transmute(This::as_any_mut as *const usize),
                into_reflect: transmute(This::into_reflect as *const usize),
                as_reflect: transmute(This::as_reflect as *const usize),
                as_reflect_mut: transmute(This::as_reflect_mut as *const usize),
                apply: transmute(This::apply as *const usize),
                try_apply: transmute(This::try_apply as *const usize),
                set: transmute(This::set as *const usize),
                reflect_kind: transmute(This::reflect_kind as *const usize),
                reflect_ref: transmute(This::reflect_ref as *const usize),
                reflect_mut: transmute(This::reflect_mut as *const usize),
                reflect_owned: transmute(This::reflect_owned as *const usize),
                clone_value: transmute(This::clone_value as *const usize),
                reflect_hash: transmute(This::reflect_hash as *const usize),
                reflect_partial_eq: transmute(This::reflect_partial_eq as *const usize),
                debug: transmute(This::debug as *const usize),
                serializable: transmute(This::serializable as *const usize),
                is_dynamic: transmute(This::is_dynamic as *const usize),
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AnyVtable<This: Sized> {
    drop_in_place: unsafe fn(*mut This),
    size: usize,
    align: usize,
    type_id: fn(*const This) -> TypeId,
}

impl<This: Sized + Any> AnyVtable<This> {
    #[allow(clippy::missing_transmute_annotations)]
    const fn new() -> AnyVtable<This> {
        unsafe {
            AnyVtable {
                drop_in_place: std::ptr::drop_in_place::<This>,
                size: std::mem::size_of::<This>(),
                align: std::mem::align_of::<This>(),
                type_id: transmute(This::type_id as *const usize),
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CommonVtable<This: Sized> {
    drop_in_place: unsafe fn(*mut This),
    size: usize,
    align: usize,
}
impl<This: Sized> CommonVtable<This> {
    const fn new() -> CommonVtable<This> {
        CommonVtable {
            drop_in_place: std::ptr::drop_in_place::<This>,
            size: std::mem::size_of::<This>(),
            align: std::mem::align_of::<This>(),
        }
    }
}

pub trait ReflectViaExt {
    fn into_reflect_via<L>(self: Box<Self>) -> Box<dyn Reflect>
    where
        Self: Sized,
        L: RemoteReflectList<Self>,
    {
        <L::RemoteReflector as RemoteReflect>::remote_into_reflect(self)
    }

    fn as_reflect_via<L>(&self) -> &dyn Reflect
    where
        Self: Sized,
        L: RemoteReflectList<Self>,
    {
        <L::RemoteReflector as RemoteReflect>::remote_as_reflect(self)
    }

    fn as_reflect_mut_via<L>(&mut self) -> &mut dyn Reflect
    where
        Self: Sized,
        L: RemoteReflectList<Self>,
    {
        <L::RemoteReflector as RemoteReflect>::remote_as_reflect_mut(self)
    }
}
impl<T> ReflectViaExt for T {}

pub trait RemoteReflectList<I> {
    type RemoteReflector: RemoteReflect<Item = I>;
}
impl<T, I> RemoteReflectList<I> for T
where
    T: RemoteReflect<Item = I>,
{
    type RemoteReflector = T;
}
