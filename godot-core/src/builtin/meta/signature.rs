/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use godot_ffi as sys;
use sys::{BuiltinMethodBind, ClassMethodBind, UtilityFunctionBind};

use crate::builtin::meta::*;
//use crate::builtin::meta::MethodParamOrReturnInfo;
use crate::builtin::{FromVariant, ToVariant, Variant};
use crate::obj::InstanceId;

#[doc(hidden)]
pub trait VarcallSignatureTuple: PtrcallSignatureTuple {
    const PARAM_COUNT: usize;

    fn param_property_info(index: usize, param_name: &str) -> PropertyInfo;
    fn param_info(index: usize, param_name: &str) -> Option<MethodParamOrReturnInfo>;
    fn return_info() -> Option<MethodParamOrReturnInfo>;

    // TODO(uninit) - can we use this for varcall/ptrcall?
    // ret: sys::GDExtensionUninitializedVariantPtr
    // ret: sys::GDExtensionUninitializedTypePtr
    unsafe fn in_varcall(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        ret: sys::GDExtensionVariantPtr,
        err: *mut sys::GDExtensionCallError,
        func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
        method_name: &str,
    );

    unsafe fn out_class_varcall(
        method_bind: sys::GDExtensionMethodBindPtr,
        method_name: &'static str,
        object_ptr: sys::GDExtensionObjectPtr,
        maybe_instance_id: Option<InstanceId>, // if not static
        args: Self::Params,
        varargs: &[Variant],
    ) -> Self::Ret;

    unsafe fn out_utility_ptrcall_varargs(
        utility_fn: UtilityFunctionBind,
        args: Self::Params,
        varargs: &[Variant],
    ) -> Self::Ret;

    fn format_args(args: &Self::Params) -> String;
}

#[doc(hidden)]
pub trait PtrcallSignatureTuple {
    type Params;
    type Ret;

    // Note: this method imposes extra bounds on GodotFfi, which may not be implemented for user types.
    // We could fall back to varcalls in such cases, and not require GodotFfi categorically.
    unsafe fn in_ptrcall(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        args_ptr: *const sys::GDExtensionConstTypePtr,
        ret: sys::GDExtensionTypePtr,
        func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
        method_name: &'static str,
        call_type: sys::PtrcallType,
    );

    unsafe fn out_class_ptrcall<Rr: PtrcallReturn<Ret = Self::Ret>>(
        method_bind: sys::GDExtensionMethodBindPtr,
        method_name: &'static str,
        object_ptr: sys::GDExtensionObjectPtr,
        maybe_instance_id: Option<InstanceId>, // if not static
        args: Self::Params,
    ) -> Self::Ret;

    unsafe fn out_builtin_ptrcall<Rr: PtrcallReturn<Ret = Self::Ret>>(
        builtin_fn: BuiltinMethodBind,
        type_ptr: sys::GDExtensionTypePtr,
        args: Self::Params,
    ) -> Self::Ret;

    unsafe fn out_utility_ptrcall(utility_fn: UtilityFunctionBind, args: Self::Params)
        -> Self::Ret;
}

// impl<P, const N: usize> Sig for [P; N]
// impl<P, T0> Sig for (T0)
// where P: VariantMetadata {
//     fn variant_type(index: usize) -> sys::GDExtensionVariantType {
//           Self[index]::
//     }
//
//     fn param_metadata(index: usize) -> sys::GDExtensionClassMethodArgumentMetadata {
//         todo!()
//     }
//
//     fn property_info(index: usize, param_name: &str) -> sys::GDExtensionPropertyInfo {
//         todo!()
//     }
// }
//

macro_rules! impl_varcall_signature_for_tuple {
    (
        $PARAM_COUNT:literal;
        $R:ident
        $(, $Pn:ident : $n:tt)* // $n cannot be literal if substituted as tuple index .0
    ) => {
        // R: FromVariantIndirect, Pn: ToVariant -> when calling engine APIs
        // R: ToVariant, Pn:
        #[allow(unused_variables)]
        impl<$R, $($Pn,)*> VarcallSignatureTuple for ($R, $($Pn,)*)
            where $R: VariantMetadata + FromVariantIndirect + ToVariant + sys::GodotFuncMarshal + Debug,
               $( $Pn: VariantMetadata + ToVariant + FromVariant + sys::GodotFuncMarshal + Debug, )*
        {
            const PARAM_COUNT: usize = $PARAM_COUNT;

            #[inline]
            fn param_info(index: usize, param_name: &str) -> Option<MethodParamOrReturnInfo> {
                match index {
                    $(
                        $n => Some($Pn::argument_info(param_name)),
                    )*
                    _ => None,
                }
            }

            #[inline]
            fn return_info() -> Option<MethodParamOrReturnInfo> {
                $R::return_info()
            }

            #[inline]
            fn param_property_info(index: usize, param_name: &str) -> PropertyInfo {
                match index {
                    $(
                        $n => $Pn::property_info(param_name),
                    )*
                    _ => unreachable!("property_info: unavailable for index {}", index),
                }
            }

            #[inline]
            unsafe fn in_varcall(
                instance_ptr: sys::GDExtensionClassInstancePtr,
                args_ptr: *const sys::GDExtensionConstVariantPtr,
                ret: sys::GDExtensionVariantPtr,
                err: *mut sys::GDExtensionCallError,
                func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
                method_name: &str,
            ) {
                $crate::out!("varcall: {}", method_name);

                let args = ($(
                    unsafe { varcall_arg::<$Pn, $n>(args_ptr, method_name) },
                )*) ;

                varcall_return::<$R>(func(instance_ptr, args), ret, err)
            }

            #[inline]
            unsafe fn out_class_varcall(
                method_bind: ClassMethodBind,
                method_name: &'static str,
                object_ptr: sys::GDExtensionObjectPtr,
                maybe_instance_id: Option<InstanceId>, // if not static
                args: Self::Params,
                varargs: &[Variant],
            ) -> Self::Ret {
                eprintln!("varcall: {method_name}");
                // Note: varcalls are not safe from failing, if the happen through an object pointer -> validity check necessary.
                if let Some(instance_id) = maybe_instance_id {
                    crate::engine::ensure_object_alive(instance_id, object_ptr, method_name);
                }

                let class_fn = sys::interface_fn!(object_method_bind_call);

                let explicit_args = [
                    $(
                        <$Pn as ToVariant>::to_variant(&args.$n),
                    )*
                ];

                let mut variant_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
                variant_ptrs.extend(explicit_args.iter().map(Variant::var_sys_const));
                variant_ptrs.extend(varargs.iter().map(Variant::var_sys_const));

                let variant = Variant::from_var_sys_init(|return_ptr| {
                    let mut err = sys::default_call_error();
                    class_fn(
                        method_bind,
                        object_ptr,
                        variant_ptrs.as_ptr(),
                        variant_ptrs.len() as i64,
                        return_ptr,
                        std::ptr::addr_of_mut!(err),
                    );

                    check_varcall_error(&err, method_name, &explicit_args, varargs);
                });
                <Self::Ret as FromVariantIndirect>::convert(variant)
            }

            // Note: this is doing a ptrcall, but uses variant conversions for it
            #[inline]
            unsafe fn out_utility_ptrcall_varargs(
                utility_fn: UtilityFunctionBind,
                args: Self::Params,
                varargs: &[Variant],
            ) -> Self::Ret {
                let explicit_args: [Variant; $PARAM_COUNT] = [
                    $(
                        <$Pn as ToVariant>::to_variant(&args.$n),
                    )*
                ];

                let mut type_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
                type_ptrs.extend(explicit_args.iter().map(sys::GodotFfi::sys_const));
                type_ptrs.extend(varargs.iter().map(sys::GodotFfi::sys_const));

                // Important: this calls from_sys_init_default().
                PtrcallReturnT::<$R>::call(|return_ptr| {
                    utility_fn(return_ptr, type_ptrs.as_ptr(), type_ptrs.len() as i32);
                })
            }

            #[inline]
            fn format_args(args: &Self::Params) -> String {
                let mut string = String::new();
                $(
                    string.push_str(&format!("{:?}, ", args.$n));
                )*
                string.remove(string.len() - 2); // remove trailing ", "
                string
            }
        }
    };
}

macro_rules! impl_ptrcall_signature_for_tuple {
    (
        $R:ident
        $(, $Pn:ident : $n:tt)* // $n cannot be literal if substituted as tuple index .0
    ) => {
        #[allow(unused_variables)]
        impl<$R, $($Pn,)*> PtrcallSignatureTuple for ($R, $($Pn,)*)
            where $R: sys::GodotFuncMarshal + Debug,
               $( $Pn: sys::GodotFuncMarshal + Debug, )*
        {
            type Params = ($($Pn,)*);
            type Ret = $R;

            #[inline]
            unsafe fn in_ptrcall(
                instance_ptr: sys::GDExtensionClassInstancePtr,
                args_ptr: *const sys::GDExtensionConstTypePtr,
                ret: sys::GDExtensionTypePtr,
                func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
                method_name: &'static str,
                call_type: sys::PtrcallType,
            ) {
                // $crate::out!("ptrcall: {}", method_name);

                let args = ($(
                    unsafe { ptrcall_arg::<$Pn, $n>(args_ptr, method_name, call_type) },
                )*) ;

                // SAFETY:
                // `ret` is always a pointer to an initialized value of type $R
                // TODO: double-check the above
                ptrcall_return::<$R>(func(instance_ptr, args), ret, method_name, call_type)
            }

            #[inline]
            unsafe fn out_class_ptrcall<Rr: PtrcallReturn<Ret = Self::Ret>>(
                method_bind: ClassMethodBind,
                method_name: &'static str,
                object_ptr: sys::GDExtensionObjectPtr,
                maybe_instance_id: Option<InstanceId>, // if not static
                args: Self::Params,
            ) -> Self::Ret {
                if let Some(instance_id) = maybe_instance_id {
                    crate::engine::ensure_object_alive(instance_id, object_ptr, method_name);
                }

                let class_fn = sys::interface_fn!(object_method_bind_ptrcall);

                #[allow(clippy::let_unit_value)]
                let marshalled_args = (
                    $(
                        <$Pn as sys::GodotFuncMarshal>::try_into_via(args.$n).unwrap(),
                    )*
                );

                let type_ptrs = [
                    $(
                        sys::GodotFfi::as_arg_ptr(&marshalled_args.$n),
                    )*
                ];

                Rr::call(|return_ptr| {
                    class_fn(method_bind, object_ptr, type_ptrs.as_ptr(), return_ptr);
                })
            }

            #[inline]
            unsafe fn out_builtin_ptrcall<Rr: PtrcallReturn<Ret = Self::Ret>>(
                builtin_fn: BuiltinMethodBind,
                type_ptr: sys::GDExtensionTypePtr,
                args: Self::Params,
            ) -> Self::Ret {
                #[allow(clippy::let_unit_value)]
                let marshalled_args = (
                    $(
                        <$Pn as sys::GodotFuncMarshal>::try_into_via(args.$n).unwrap(),
                    )*
                );

                let type_ptrs = [
                    $(
                        sys::GodotFfi::as_arg_ptr(&marshalled_args.$n),
                    )*
                ];

                Rr::call(|return_ptr| {
                    builtin_fn(type_ptr, type_ptrs.as_ptr(), return_ptr, type_ptrs.len() as i32);
                })
            }

            #[inline]
            unsafe fn out_utility_ptrcall(
                utility_fn: UtilityFunctionBind,
                args: Self::Params,
            ) -> Self::Ret {
                #[allow(clippy::let_unit_value)]
                let marshalled_args = (
                    $(
                        <$Pn as sys::GodotFuncMarshal>::try_into_via(args.$n).unwrap(),
                    )*
                );

                let arg_ptrs = [
                    $(
                        sys::GodotFfi::as_arg_ptr(&marshalled_args.$n),
                    )*
                ];

                PtrcallReturnT::<$R>::call(|return_ptr| {
                    utility_fn(return_ptr, arg_ptrs.as_ptr(), arg_ptrs.len() as i32);
                })
            }
        }
    };
}

/// Convert the `N`th argument of `args_ptr` into a value of type `P`.
///
/// # Safety
/// - It must be safe to dereference the pointer at `args_ptr.offset(N)` .
unsafe fn varcall_arg<P: FromVariant, const N: isize>(
    args_ptr: *const sys::GDExtensionConstVariantPtr,
    method_name: &str,
) -> P {
    let variant = &*(*args_ptr.offset(N) as *mut Variant); // TODO from_var_sys
    P::try_from_variant(variant)
        .unwrap_or_else(|_| param_error::<P>(method_name, N as i32, variant))
}

/// Moves `ret_val` into `ret`.
///
/// # Safety
/// - `ret` must be a pointer to an initialized `Variant`.
/// - It must be safe to write a `Variant` once to `ret`.
/// - It must be safe to write a `sys::GDExtensionCallError` once to `err`.
unsafe fn varcall_return<R: ToVariant>(
    ret_val: R,
    ret: sys::GDExtensionVariantPtr,
    err: *mut sys::GDExtensionCallError,
) {
    let ret_variant = ret_val.to_variant(); // TODO write_sys
    *(ret as *mut Variant) = ret_variant;
    (*err).error = sys::GDEXTENSION_CALL_OK;
}

/// Convert the `N`th argument of `args_ptr` into a value of type `P`.
///
/// # Safety
/// - It must be safe to dereference the address at `args_ptr.offset(N)` .
/// - The pointer at `args_ptr.offset(N)` must follow the safety requirements as laid out in
///   [`GodotFuncMarshal::try_from_arg`][sys::GodotFuncMarshal::try_from_arg].
unsafe fn ptrcall_arg<P: sys::GodotFuncMarshal, const N: isize>(
    args_ptr: *const sys::GDExtensionConstTypePtr,
    method_name: &str,
    call_type: sys::PtrcallType,
) -> P {
    P::try_from_arg(sys::force_mut_ptr(*args_ptr.offset(N)), call_type)
        .unwrap_or_else(|e| param_error::<P>(method_name, N as i32, &e))
}

/// Moves `ret_val` into `ret`.
///
/// # Safety
/// `ret_val`, `ret`, and `call_type` must follow the safety requirements as laid out in
/// [`GodotFuncMarshal::try_return`](sys::GodotFuncMarshal::try_return).
unsafe fn ptrcall_return<R: sys::GodotFuncMarshal + std::fmt::Debug>(
    ret_val: R,
    ret: sys::GDExtensionTypePtr,
    method_name: &str,
    call_type: sys::PtrcallType,
) {
    ret_val
        .try_return(ret, call_type)
        .unwrap_or_else(|ret_val| return_error::<R>(method_name, &ret_val))
}

fn param_error<P>(method_name: &str, index: i32, arg: &impl Debug) -> ! {
    let param_ty = std::any::type_name::<P>();
    panic!(
        "{method_name}: parameter [{index}] has type {param_ty}, which is unable to store argument {arg:?}",
    );
}

fn return_error<R>(method_name: &str, arg: &impl Debug) -> ! {
    let return_ty = std::any::type_name::<R>();
    panic!("{method_name}: return type {return_ty} is unable to store value {arg:?}",);
}

fn check_varcall_error<T>(
    err: &sys::GDExtensionCallError,
    fn_name: &str,
    explicit_args: &[T],
    varargs: &[Variant],
) where
    T: Debug + ToVariant,
{
    if err.error == sys::GDEXTENSION_CALL_OK {
        return;
    }

    // TODO(optimize): split into non-generic, expensive parts after error check

    let mut arg_types = Vec::with_capacity(explicit_args.len() + varargs.len());
    arg_types.extend(explicit_args.iter().map(|arg| arg.to_variant().get_type()));
    arg_types.extend(varargs.iter().map(Variant::get_type));

    let explicit_args_str = join_to_string(explicit_args);
    let vararg_str = join_to_string(varargs);

    let func_str = format!("{fn_name}({explicit_args_str}; varargs {vararg_str})");

    sys::panic_call_error(err, &func_str, &arg_types);
}

fn join_to_string<T: Debug>(list: &[T]) -> String {
    list.iter()
        .map(|v| format!("{v:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Helper trait to support `()` which doesn't implement `FromVariant`.
trait FromVariantIndirect {
    fn convert(variant: Variant) -> Self;
}

impl FromVariantIndirect for () {
    fn convert(_variant: Variant) -> Self {}
}

impl<T: FromVariant> FromVariantIndirect for T {
    fn convert(variant: Variant) -> Self {
        T::from_variant(&variant)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Poor man's variadic templates.
// For example, RenderingServer::environment_set_volumetric_fog() has 14 parameters. We may need to extend this if the API adds more such methods.

impl_varcall_signature_for_tuple!(0; R);
impl_varcall_signature_for_tuple!(1; R, P0: 0);
impl_varcall_signature_for_tuple!(2; R, P0: 0, P1: 1);
impl_varcall_signature_for_tuple!(3; R, P0: 0, P1: 1, P2: 2);
impl_varcall_signature_for_tuple!(4; R, P0: 0, P1: 1, P2: 2, P3: 3);
impl_varcall_signature_for_tuple!(5; R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4);
impl_varcall_signature_for_tuple!(6; R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5);
impl_varcall_signature_for_tuple!(7; R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6);
impl_varcall_signature_for_tuple!(8; R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7);
impl_varcall_signature_for_tuple!(9; R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8);
impl_varcall_signature_for_tuple!(10; R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9);
impl_varcall_signature_for_tuple!(11; R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9, P10: 10);
impl_varcall_signature_for_tuple!(12; R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9, P10: 10, P11: 11);
impl_varcall_signature_for_tuple!(13; R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9, P10: 10, P11: 11, P12: 12);
impl_varcall_signature_for_tuple!(14; R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9, P10: 10, P11: 11, P12: 12, P13: 13);

impl_ptrcall_signature_for_tuple!(R);
impl_ptrcall_signature_for_tuple!(R, P0: 0);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9, P10: 10);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9, P10: 10, P11: 11);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9, P10: 10, P11: 11, P12: 12);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9, P10: 10, P11: 11, P12: 12, P13: 13);
