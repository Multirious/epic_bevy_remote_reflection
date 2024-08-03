use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::mem::{transmute_copy, ManuallyDrop};
use std::sync::{OnceLock, RwLock};

use bevy_reflect::{
    serde::Serializable, ApplyError, Reflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef,
    TypeInfo,
};

static CUSTOM_REFLECT_VTABLES: OnceLock<RwLock<HashMap<TypeId, &'static ReflectVtable>>> =
    OnceLock::new();

unsafe fn change_reflect_vtable<R: RemoteReflect>(vtable: *mut ReflectVtable) {
    (*vtable).type_id = |_| TypeId::of::<R::Item>();
    (*vtable).into_any = |this: Box<()>| -> Box<dyn Any> {
        transmute_copy::<_, Box<R::Item>>(&ManuallyDrop::new(this))
    };
    (*vtable).as_any = |this: *const ()| -> *const dyn Any {
        let this: &dyn Any = transmute_copy::<_, &R::Item>(&&*this);
        this as _
    };
    (*vtable).as_any_mut = |this: *mut ()| -> *mut dyn Any {
        let this: &mut dyn Any = transmute_copy::<_, &mut R::Item>(&&*this);
        this as _
    };
    (*vtable).into_reflect = |this: Box<()>| -> Box<dyn Reflect> {
        let this: Box<R> = transmute_copy(&ManuallyDrop::new(this));
        let mut this: Box<dyn Reflect> = this;
        let this_raw = &mut this as *mut _ as *mut DynReflect;
        let vtable = *CUSTOM_REFLECT_VTABLES
            .get()
            .unwrap()
            .read()
            .unwrap()
            .get(&TypeId::of::<R>())
            .unwrap();
        (*this_raw).vtable = vtable as *const _ as *mut _;
        this
    };
    (*vtable).as_reflect = |this: *const ()| -> *const dyn Reflect {
        let mut this: &dyn Reflect = transmute_copy::<_, &R>(&&*this);
        let this_raw = &mut this as *mut _ as *mut DynReflect;
        let vtable = *CUSTOM_REFLECT_VTABLES
            .get()
            .unwrap()
            .read()
            .unwrap()
            .get(&TypeId::of::<R>())
            .unwrap();
        (*this_raw).vtable = vtable as *const _ as *mut _;
        this
    };
    (*vtable).as_reflect_mut = |this: *mut ()| -> *mut dyn Reflect {
        let mut this: &mut dyn Reflect = transmute_copy::<_, &mut R>(&&*this);
        let this_raw = &mut this as *mut _ as *mut DynReflect;
        let vtable = *CUSTOM_REFLECT_VTABLES
            .get()
            .unwrap()
            .read()
            .unwrap()
            .get(&TypeId::of::<R>())
            .unwrap();
        (*this_raw).vtable = vtable as *const _ as *mut _;
        this
    };
}

unsafe fn get_or_new_custom_reflect_vtable<R: RemoteReflect>(
    existing_table: *const ReflectVtable,
) -> &'static ReflectVtable {
    CUSTOM_REFLECT_VTABLES
        .get_or_init(|| RwLock::new(HashMap::new()))
        .write()
        .unwrap()
        .entry(TypeId::of::<R>())
        .or_insert_with(|| {
            let new_reflect_vtable = Box::leak(Box::new(*existing_table));
            change_reflect_vtable::<R>(new_reflect_vtable);
            new_reflect_vtable
        })
}

/// # Safety
/// This trait must only implement on a remote type wrapper that is structured
/// exactly like the remote type.
pub unsafe trait RemoteReflect: Reflect + Sized {
    /// The remote type
    type Item: Send + Sync + 'static;

    fn remote_into_reflect(item: Box<Self::Item>) -> Box<dyn Reflect> {
        // Swapping the virtual table inside the trait object to make the remote
        // type wrapper acts exactly like the remote type.
        unsafe {
            let mut v: Box<dyn Reflect> = transmute_copy::<_, Box<Self>>(&ManuallyDrop::new(item));
            let dyn_reflect = &mut v as *mut _ as *mut DynReflect;
            let existing_table = (*dyn_reflect).vtable;
            let vtable = get_or_new_custom_reflect_vtable::<Self>(existing_table);
            (*dyn_reflect).vtable = vtable as *const _ as *mut _;
            v
        }
    }

    fn remote_as_reflect(item: &Self::Item) -> &dyn Reflect {
        unsafe {
            let mut v: &dyn Reflect = transmute_copy::<_, &Self>(&item);
            let dyn_reflect = &mut v as *mut _ as *mut DynReflect;
            let existing_table = (*dyn_reflect).vtable;
            let vtable = get_or_new_custom_reflect_vtable::<Self>(existing_table);
            (*dyn_reflect).vtable = vtable as *const _ as *mut _;
            v
        }
    }

    fn remote_as_reflect_mut(item: &mut Self::Item) -> &mut dyn Reflect {
        unsafe {
            let mut v: &mut dyn Reflect = transmute_copy::<_, &mut Self>(&item);
            let dyn_reflect = &mut v as *mut _ as *mut DynReflect;
            let existing_table = (*dyn_reflect).vtable;
            let vtable = get_or_new_custom_reflect_vtable::<Self>(existing_table);
            (*dyn_reflect).vtable = vtable as *const _ as *mut _;
            v
        }
    }
}

#[repr(C)]
#[derive(Clone)]
struct DynReflect {
    data: *const (),
    vtable: *mut ReflectVtable,
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(clippy::type_complexity)]
struct ReflectVtable {
    drop_in_place: fn(*const ()),
    size: usize,
    align: usize,
    reflect_type_path: fn(*const ()) -> *const str,
    reflect_short_type_path: fn(*const ()) -> *const str,
    reflect_type_ident: fn(*const ()) -> *const str,
    reflect_crate_name: fn(*const ()) -> Option<*const str>,
    reflect_module_path: fn(*const ()) -> Option<*const str>,
    type_id: fn(*const ()) -> TypeId,
    _unknown_1: usize,
    _unknown_2: usize,
    _unknown_3: usize,
    get_represented_type_info: fn(*const ()) -> Option<&'static TypeInfo>,
    into_any: fn(Box<()>) -> Box<dyn Any>,
    as_any: fn(*const ()) -> *const dyn Any,
    as_any_mut: fn(*mut ()) -> *mut dyn Any,
    into_reflect: fn(Box<()>) -> Box<dyn Reflect>,
    as_reflect: fn(*const ()) -> *const dyn Reflect,
    as_reflect_mut: fn(*mut ()) -> *mut dyn Reflect,
    apply: fn(*mut (), *const dyn Reflect),
    try_apply: fn(*mut (), *const dyn Reflect) -> Result<(), ApplyError>,
    set: fn(*mut (), Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>,
    reflect_kind: fn(*const ()) -> ReflectKind,
    reflect_ref: fn(*const ()) -> ReflectRef<'static>,
    reflect_mut: fn(*mut ()) -> ReflectMut<'static>,
    reflect_owned: fn(Box<()>) -> ReflectOwned,
    clone_value: fn(*const ()) -> Box<dyn Reflect>,
    reflect_hash: fn(*const ()) -> Option<u64>,
    reflect_partial_eq: fn(*const (), &dyn Reflect) -> Option<bool>,
    debug: fn(*const (), *mut std::fmt::Formatter<'_>) -> std::fmt::Result,
    serializable: fn(*const ()) -> Option<Serializable<'static>>,
    is_dynamicfn: fn(*const ()) -> bool,
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
