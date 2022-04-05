use serde::Deserialize;
use swc_plugin::{ast::*, plugin_transform, TransformPluginProgramMetadata};

#[derive(Deserialize)]
struct CustomName {
    format: String,
}

#[derive(Deserialize)]
struct CustomStylePath {
    format: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CustomNameOption {
    String(String),
    Format(CustomName),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StyleOption {
    Bool(bool),
    CSS,
    Format(CustomStylePath),
}

impl Default for StyleOption {
    fn default() -> Self {
        StyleOption::Bool(true)
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Options {
    library_name: String,
    #[serde(default = "default_library_directory")]
    library_directory: String,
    custom_name: Option<CustomNameOption>,
    style_library_directory: Option<String>,
    #[serde(default = "default_camel_2_dash_component_name")]
    camel_2_dash_component_name: bool,
    #[serde(default)]
    transform_to_default_import: bool,
    #[serde(default)]
    style: StyleOption,
}

fn default_library_directory() -> String {
    "lib".to_string()
}

fn default_camel_2_dash_component_name() -> bool {
    true
}

pub struct TransformVisitor {
    options: Options,
    import_items: Vec<ImportDecl>,
}

impl VisitMut for TransformVisitor {
    // Implement necessary visit_mut_* methods for actual custom transform.
    // A comprehensive list of possible visitor methods can be found here:
    // https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html
    fn visit_mut_module(&mut self, m: &mut Module) {
        for i in (0..m.body.len()).rev() {
            if let Some(decl) = m.body[i].as_module_decl() {
                if let Some(import_decl) = decl.as_import() {
                    if import_decl.src.value == self.options.library_name
                        && import_decl.specifiers.is_empty()
                    {
                        m.body.remove(i);
                    }
                }
            }
        }
        m.body = self
            .import_items
            .iter()
            .map(|item| ModuleItem::ModuleDecl(ModuleDecl::Import(item.clone())))
            .chain(m.body.drain(..))
            .collect();
    }

    fn visit_mut_import_decl(&mut self, n: &mut ImportDecl) {
        if n.src.value == self.options.library_name {
            for i in (0..n.specifiers.len()).rev() {
                if let Some(specifier) = n.specifiers[i].as_named() {
                    let imported = if let Some(ModuleExportName::Ident(ref name)) = specifier.imported {
                        name.sym.clone()
                    } else {
                        specifier.local.sym.clone()
                    };
                }
            }
        }
    }
}

/// An example plugin function with macro support.
/// `plugin_transform` macro interop pointers into deserialized structs, as well
/// as returning ptr back to host.
///
/// It is possible to opt out from macro by writing transform fn manually via
/// `__plugin_process_impl(
///     ast_ptr: *const u8,
///     ast_ptr_len: i32,
///     config_str_ptr: *const u8,
///     config_str_ptr_len: i32,
///     context_str_ptr: *const u8,
///     context_str_ptr_len: i32) ->
///     i32 /*  0 for success, fail otherwise.
///             Note this is only for internal pointer interop result,
///             not actual transform result */
///
/// if plugin need to handle low-level ptr directly. However, there are
/// important steps manually need to be performed like sending transformed
/// results back to host. Refer swc_plugin_macro how does it work internally.
#[plugin_transform]
pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    let options = if let Ok(opts) = serde_json::from_str::<Options>(&_metadata.plugin_config) {
        opts
    } else {
        Default::default()
    };
    program.fold_with(&mut as_folder(TransformVisitor {
        import_items: vec![],
        options: options,
    }))
}
