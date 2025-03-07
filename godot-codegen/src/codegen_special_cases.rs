/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Codegen-dependent exclusions. Can be removed if feature `codegen-full` is removed.

use crate::api_parser::{BuiltinClassMethod, ClassMethod, UtilityFunction};
use crate::context::Context;
use crate::{special_cases, TyName};

pub(crate) fn is_builtin_method_excluded(method: &BuiltinClassMethod) -> bool {
    // Builtin class methods that need varcall are not currently available in GDExtension.
    // See https://github.com/godot-rust/gdext/issues/382.
    method.is_vararg
}

#[cfg(not(feature = "codegen-full"))]
pub(crate) fn is_class_excluded(class: &str) -> bool {
    !SELECTED_CLASSES.contains(&class)
}

#[cfg(feature = "codegen-full")]
pub(crate) fn is_class_excluded(_class: &str) -> bool {
    false
}

#[cfg(not(feature = "codegen-full"))]
fn is_type_excluded(ty: &str, ctx: &mut Context) -> bool {
    use crate::{util, RustTy};

    fn is_rust_type_excluded(ty: &RustTy) -> bool {
        match ty {
            RustTy::BuiltinIdent(_) => false,
            RustTy::BuiltinArray(_) => false,
            RustTy::RawPointer { inner, .. } => is_rust_type_excluded(inner),
            RustTy::EngineArray { elem_class, .. } => is_class_excluded(elem_class.as_str()),
            RustTy::EngineEnum {
                surrounding_class, ..
            } => match surrounding_class.as_ref() {
                None => false,
                Some(class) => is_class_excluded(class.as_str()),
            },
            RustTy::EngineClass { inner_class, .. } => is_class_excluded(&inner_class.to_string()),
        }
    }
    is_rust_type_excluded(&util::to_rust_type(ty, None, ctx))
}

pub(crate) fn is_method_excluded(
    method: &ClassMethod,
    is_virtual_impl: bool,
    ctx: &mut Context,
) -> bool {
    let is_arg_or_return_excluded = |ty: &str, _ctx: &mut Context| {
        let class_deleted = special_cases::is_class_deleted(&TyName::from_godot(ty));

        #[cfg(not(feature = "codegen-full"))]
        {
            class_deleted || is_type_excluded(ty, _ctx)
        }
        #[cfg(feature = "codegen-full")]
        {
            class_deleted
        }
    };

    // Exclude if return type contains an excluded type.
    if method.return_value.as_ref().map_or(false, |ret| {
        is_arg_or_return_excluded(ret.type_.as_str(), ctx)
    }) {
        return true;
    }

    // Exclude if any argument contains an excluded type.
    if method.arguments.as_ref().map_or(false, |args| {
        args.iter()
            .any(|arg| is_arg_or_return_excluded(arg.type_.as_str(), ctx))
    }) {
        return true;
    }

    // Virtual methods are not part of the class API itself, but exposed as an accompanying trait.
    if !is_virtual_impl && method.name.starts_with('_') {
        return true;
    }

    false
}

#[cfg(feature = "codegen-full")]
pub(crate) fn is_function_excluded(_function: &UtilityFunction, _ctx: &mut Context) -> bool {
    false
}

#[cfg(not(feature = "codegen-full"))]
pub(crate) fn is_function_excluded(function: &UtilityFunction, ctx: &mut Context) -> bool {
    function
        .return_type
        .as_ref()
        .map_or(false, |ret| is_type_excluded(ret.as_str(), ctx))
        || function.arguments.as_ref().map_or(false, |args| {
            args.iter()
                .any(|arg| is_type_excluded(arg.type_.as_str(), ctx))
        })
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Allowed-classes

// Classes for minimal config
#[cfg(not(feature = "codegen-full"))]
const SELECTED_CLASSES: &[&str] = &[
    "AnimatedSprite2D",
    "ArrayMesh",
    "Area2D",
    "AudioStreamPlayer",
    "BaseButton",
    "Button",
    "BoxMesh",
    "Camera2D",
    "Camera3D",
    "CanvasItem",
    "CanvasLayer",
    "ClassDB",
    "CollisionObject2D",
    "CollisionShape2D",
    "Control",
    "Engine",
    "FileAccess",
    "HTTPRequest",
    "Image",
    "ImageTextureLayered",
    "Input",
    "InputEvent",
    "InputEventAction",
    "Label",
    "MainLoop",
    "Marker2D",
    "Mesh",
    "Node",
    "Node2D",
    "Node3D",
    "Node3DGizmo",
    "Object",
    "OS",
    "PackedScene",
    "PathFollow2D",
    "PhysicsBody2D",
    "PrimitiveMesh",
    "RefCounted",
    "RenderingServer",
    "Resource",
    "ResourceFormatLoader",
    "ResourceLoader",
    "RigidBody2D",
    "SceneTree",
    "Sprite2D",
    "SpriteFrames",
    "TextServer",
    "TextServerExtension",
    "Texture",
    "Texture2DArray",
    "TextureLayered",
    "Time",
    "Timer",
    "Window",
    "Viewport",
];
