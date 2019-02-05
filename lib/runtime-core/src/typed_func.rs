use crate::{
    error::RuntimeError,
    export::{Context, Export, FuncPointer},
    import::IsExport,
    types::{FuncSig, Type, WasmExternType},
    vm::Ctx,
};
use std::{marker::PhantomData, mem, ptr, sync::Arc};

pub trait Safeness {}
pub struct Safe;
pub struct Unsafe;
impl Safeness for Safe {}
impl Safeness for Unsafe {}

pub trait WasmTypeList {
    type CStruct;
    fn from_c_struct(c_struct: Self::CStruct) -> Self;
    fn into_c_struct(self) -> Self::CStruct;
    fn types() -> &'static [Type];
    unsafe fn call<Rets>(self, f: *const (), ctx: *mut Ctx) -> Rets
    where
        Rets: WasmTypeList;
}

pub trait ExternalFunction<Args, Rets>
where
    Args: WasmTypeList,
    Rets: WasmTypeList,
{
    fn to_raw(self) -> *const ();
}

pub struct Func<'a, Args = (), Rets = (), Safety: Safeness = Safe> {
    f: *const (),
    ctx: *mut Ctx,
    _phantom: PhantomData<(&'a (), Safety, Args, Rets)>,
}

impl<'a, Args, Rets> Func<'a, Args, Rets, Safe>
where
    Args: WasmTypeList,
    Rets: WasmTypeList,
{
    pub(crate) unsafe fn new_from_ptr(f: *const (), ctx: *mut Ctx) -> Func<'a, Args, Rets, Safe> {
        Func {
            f,
            ctx,
            _phantom: PhantomData,
        }
    }
}

impl<'a, Args, Rets> Func<'a, Args, Rets, Unsafe>
where
    Args: WasmTypeList,
    Rets: WasmTypeList,
{
    pub fn new<F>(f: F) -> Func<'a, Args, Rets, Unsafe>
    where
        F: ExternalFunction<Args, Rets>,
    {
        Func {
            f: f.to_raw(),
            ctx: ptr::null_mut(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, Args, Rets, Safety> Func<'a, Args, Rets, Safety>
where
    Args: WasmTypeList,
    Rets: WasmTypeList,
    Safety: Safeness,
{
    pub fn params(&self) -> &'static [Type] {
        Args::types()
    }
    pub fn returns(&self) -> &'static [Type] {
        Rets::types()
    }
}

impl<A: WasmExternType> WasmTypeList for (A,) {
    type CStruct = S1<A>;
    fn from_c_struct(c_struct: Self::CStruct) -> Self {
        let S1(a) = c_struct;
        (a,)
    }
    fn into_c_struct(self) -> Self::CStruct {
        #[allow(unused_parens, non_snake_case)]
        let (a,) = self;
        S1(a)
    }
    fn types() -> &'static [Type] {
        &[A::TYPE]
    }
    #[allow(non_snake_case)]
    unsafe fn call<Rets: WasmTypeList>(self, f: *const (), ctx: *mut Ctx) -> Rets {
        let f: extern "C" fn(A, *mut Ctx) -> Rets = mem::transmute(f);
        let (a,) = self;
        f(a, ctx)
    }
}

impl<'a, A: WasmExternType, Rets> Func<'a, (A,), Rets, Safe>
where
    Rets: WasmTypeList,
{
    pub fn call(&self, a: A) -> Result<Rets, RuntimeError> {
        Ok(unsafe { <A as WasmTypeList>::call(a, self.f, self.ctx) })
    }
}

macro_rules! impl_traits {
    ( $struct_name:ident, $( $x:ident ),* ) => {
        #[repr(C)]
        pub struct $struct_name <$( $x ),*> ( $( $x ),* );

        impl< $( $x: WasmExternType, )* > WasmTypeList for ( $( $x ),* ) {
            type CStruct = $struct_name<$( $x ),*>;
            fn from_c_struct(c_struct: Self::CStruct) -> Self {
                #[allow(non_snake_case)]
                let $struct_name ( $( $x ),* ) = c_struct;
                ( $( $x ),* )
            }
            fn into_c_struct(self) -> Self::CStruct {
                #[allow(unused_parens, non_snake_case)]
                let ( $( $x ),* ) = self;
                $struct_name ( $( $x ),* )
            }
            fn types() -> &'static [Type] {
                &[$( $x::TYPE, )*]
            }
            #[allow(non_snake_case)]
            unsafe fn call<Rets: WasmTypeList>(self, f: *const (), ctx: *mut Ctx) -> Rets {
                let f: extern fn( $( $x, )* *mut Ctx) -> Rets::CStruct = mem::transmute(f);
                #[allow(unused_parens)]
                let ( $( $x ),* ) = self;
                let c_struct = f( $( $x, )* ctx);
                Rets::from_c_struct(c_struct)
            }
        }

        impl< $( $x: WasmExternType, )* Rets: WasmTypeList, FN: Fn( $( $x, )* &mut Ctx) -> Rets> ExternalFunction<($( $x ),*), Rets> for FN {
            #[allow(non_snake_case)]
            fn to_raw(self) -> *const () {
                assert_eq!(mem::size_of::<Self>(), 0, "you cannot use a closure that captures state for `Func`.");

                extern fn wrap<$( $x: WasmExternType, )* Rets: WasmTypeList, FN: Fn( $( $x, )* &mut Ctx) -> Rets>( $( $x: $x, )* ctx: &mut Ctx) -> Rets::CStruct {
                    let f: FN = unsafe { mem::transmute_copy(&()) };
                    let rets = f( $( $x, )* ctx);
                    rets.into_c_struct()
                }

                wrap::<$( $x, )* Rets, Self> as *const ()
            }
        }

        impl<'a, $( $x: WasmExternType, )* Rets> Func<'a, ( $( $x ),* ), Rets, Safe>
        where
            Rets: WasmTypeList,
        {
            #[allow(non_snake_case)]
            pub fn call(&self, $( $x: $x, )* ) -> Result<Rets, RuntimeError> {
                #[allow(unused_parens)]
                Ok(unsafe { <( $( $x ),* ) as WasmTypeList>::call(( $($x),* ), self.f, self.ctx) })
            }
        }
    };
}

impl_traits!(S0,);
impl_traits!(S1, A);
impl_traits!(S2, A, B);
impl_traits!(S3, A, B, C);
impl_traits!(S4, A, B, C, D);
impl_traits!(S5, A, B, C, D, E);
impl_traits!(S6, A, B, C, D, E, F);
impl_traits!(S7, A, B, C, D, E, F, G);
impl_traits!(S8, A, B, C, D, E, F, G, H);
impl_traits!(S9, A, B, C, D, E, F, G, H, I);
impl_traits!(S10, A, B, C, D, E, F, G, H, I, J);
impl_traits!(S11, A, B, C, D, E, F, G, H, I, J, K);

impl<'a, Args, Rets, Safety> IsExport for Func<'a, Args, Rets, Safety>
where
    Args: WasmTypeList,
    Rets: WasmTypeList,
    Safety: Safeness,
{
    fn to_export(&self) -> Export {
        let func = unsafe { FuncPointer::new(self.f as _) };
        let ctx = Context::Internal;
        let signature = Arc::new(FuncSig::new(Args::types(), Rets::types()));

        Export::Function {
            func,
            ctx,
            signature,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_call() {
        fn foo(a: i32, b: i32, _ctx: &mut Ctx) -> (i32, i32) {
            (a, b)
        }

        let _f = Func::new(foo);
    }

    #[test]
    fn test_imports() {
        use crate::{func, imports};

        fn foo(a: i32, _ctx: &mut Ctx) -> i32 {
            a
        }

        let _import_object = imports! {
            "env" => {
                "foo" => func!(foo),
            },
        };
    }
}
