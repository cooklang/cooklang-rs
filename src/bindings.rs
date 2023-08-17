use crate::*;



#[derive(uniffi::Record)]
pub struct CooklangRecipe {
    name: String
}


#[derive(uniffi::Record)]
pub struct Ingredient {
    name: String,
    quantity: String,
    units: String,
}

#[uniffi::export]
pub fn parse(input: String, recipe_name: String) -> CooklangRecipe {
    let parser = CooklangParser::new(Extensions::empty(), Converter::empty());

    let ast = parser::parse(&input, parser.extensions).take_output().unwrap();
    let result = analysis::parse_ast(ast, parser.extensions, &parser.converter, None)
        .take_output()
        .unwrap();

    CooklangRecipe {
            name: recipe_name.to_string(),
            // metadata: result.metadata,
            // sections: result.sections,
            ingredients: result.ingredients,
            // cookware: result.cookware,
            // timers: result.timers,
            // inline_quantities: result.inline_quantities,
            // data: (),
    }
}

// type UT = crate::UniFfiTag;


// unsafe impl FfiConverter<UT> for Recipe {
//     ffi_converter_rust_buffer_lift_and_lower!(UT);
//     ffi_converter_default_return!(UT);

//     fn write(obj: Recipe, buf: &mut Vec<u8>) {

//     }

//     fn try_read(buf: &mut &[u8]) -> anyhow::Result<Recipe> {
//         Ok(Recipe {})
//     }

//     const TYPE_ID_META: MetadataBuffer = MetadataBuffer::from_code(metadata::codes::TYPE_INTERFACE);
// }


// unsafe impl FfiConverter<UT> for &str {
//     ffi_converter_rust_buffer_lift_and_lower!(UT);
//     ffi_converter_default_return!(UT);

//     fn write(obj: &str, buf: &mut Vec<u8>) {

//     }

//     fn try_read(buf: &mut &[u8]) -> anyhow::Result<&str> {
//         Ok()
//     }

//     const TYPE_ID_META: MetadataBuffer = MetadataBuffer::from_code(metadata::codes::TYPE_STRING);
// }

uniffi::setup_scaffolding!();