use enumflags2::make_bitflags;
use std::sync::Arc;
use ze_imgui::ze_imgui_sys::ImVec2;
use ze_imgui::{Context, TableColumnFlagBits, TableColumnFlags, TableFlagBits};
use ze_reflection::{
    MetaAttributeValue, PrimitiveType, Reflectable, TypeDataDescription, TypeDescription,
};

/// Draw a property editor using reflection
/// Returns whether or not something has changed
pub fn draw_property_editor<T: Reflectable>(imgui: &mut Context, object: &mut T) -> bool {
    imgui.begin_table(
        "PropertiesTable",
        2,
        make_bitflags! { TableFlagBits::{Resizable | NoBordersInBodyUntilResize} },
        imgui.available_content_region(),
    );

    imgui.table_setup_column(
        "Name",
        0.25,
        TableColumnFlags::from_flag(TableColumnFlagBits::WidthStretch),
    );

    imgui.table_setup_column(
        "Value",
        0.75,
        TableColumnFlags::from_flag(TableColumnFlagBits::WidthStretch),
    );

    let modified =
        draw_property_editor_internal(imgui, &T::type_desc(), object as *mut _ as *mut u8, "obj");

    imgui.end_table();

    modified
}

fn draw_property_editor_internal(
    imgui: &mut Context,
    type_desc: &Arc<TypeDescription>,
    value: *mut u8,
    label: &str,
) -> bool {
    match type_desc.data() {
        TypeDataDescription::Primitive(primitive_type) => match primitive_type {
            PrimitiveType::Bool => {
                let value = unsafe { (value as *mut bool).as_mut().unwrap_unchecked() };
                imgui.checkbox(label, value)
            }
            _ => {
                todo!()
            }
        },
        TypeDataDescription::Struct(struct_desc) => {
            let mut field_modified = false;

            for field in struct_desc.fields() {
                imgui.table_next_row();
                imgui.dummy(ImVec2::new(5.0, 5.0));
                imgui.table_next_column();

                let display_name = field.attributes().attribute("display_name").map_or(
                    field.name().to_string(),
                    |a| match a.value() {
                        None => field.name().to_string(),
                        Some(val) => match val {
                            MetaAttributeValue::Value(val) => val.clone(),
                            MetaAttributeValue::List(_) => field.name().to_string(),
                        },
                    },
                );

                imgui.text(&display_name);
                imgui.table_next_column();
                if draw_property_editor_internal(
                    imgui,
                    field.ty(),
                    unsafe { value.add(field.offset_in_bytes()) },
                    &format!("##{}", field.name()),
                ) {
                    field_modified = true;
                }
            }

            field_modified
        }
        TypeDataDescription::Enum(enum_desc) => {
            let current_variant = enum_desc.variant_of_ptr(value).unwrap();
            let mut modified = false;
            if imgui.begin_combo(label, current_variant.name()) {
                for variant in enum_desc.variants() {
                    if imgui.selectable(variant.name(), ImVec2::default()) {
                        enum_desc.set_variant_of_ptr(value, variant.discriminant());
                        modified = true;
                    }
                }

                imgui.end_combo();
            }

            modified
        }
    }
}
