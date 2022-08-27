use enumflags2::make_bitflags;
use std::sync::Arc;
use ze_imgui::ze_imgui_sys::ImVec2;
use ze_imgui::{Context, TableColumnFlagBits, TableColumnFlags, TableFlagBits};
use ze_reflection::{PrimitiveType, Reflectable, TypeDataDescription, TypeDescription};

pub fn draw_property_editor<T: Reflectable>(imgui: &mut Context, object: &mut T) {
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

    draw_property_editor_internal(imgui, &T::type_desc(), object as *mut _ as *mut u8, "obj");

    imgui.end_table();
}

fn draw_property_editor_internal(
    imgui: &mut Context,
    type_desc: &Arc<TypeDescription>,
    value: *mut u8,
    label: &str,
) {
    match type_desc.data() {
        TypeDataDescription::Primitive(primitive_type) => match primitive_type {
            PrimitiveType::Bool => {
                let value = unsafe { (value as *mut bool).as_mut().unwrap_unchecked() };
                imgui.checkbox(label, value);
            }
            _ => imgui.text(&format!("Unsupported primitive {:?}", primitive_type)),
        },
        TypeDataDescription::Struct(struct_desc) => {
            for field in struct_desc.fields() {
                imgui.table_next_row();
                imgui.dummy(ImVec2::new(5.0, 5.0));
                imgui.table_next_column();
                imgui.text(field.name());
                imgui.table_next_column();
                draw_property_editor_internal(
                    imgui,
                    field.ty(),
                    unsafe { value.add(field.offset_in_bytes()) },
                    &format!("##{}", field.name()),
                );
            }
        }
        TypeDataDescription::Enum(enum_desc) => {
            let current_variant = enum_desc.variant_of_ptr(value).unwrap();
            if imgui.begin_combo(label, current_variant.name()) {
                for variant in enum_desc.variants() {
                    if imgui.selectable(variant.name(), ImVec2::default()) {
                        enum_desc.set_variant_of_ptr(value, variant.discriminant())
                    }
                }

                imgui.end_combo();
            }
        }
    };
}
