use regex::{Captures, Regex};
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
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StyleOption {
    Bool(bool),
    CSS,
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
    #[serde(default = "default_transform_to_default_import")]
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

fn default_transform_to_default_import() -> bool {
    true
}

fn is_double_quote(s: &str) -> bool {
    s.chars().next().unwrap() == '"'
}

pub struct TransformVisitor {
    options: Options,
    import_items: Vec<ImportDecl>,
}

impl TransformVisitor {
    fn generate_component_path<'a>(&self, source: &'a str) -> String {
        if let Some(custom_name) = &self.options.custom_name {
            match custom_name {
                CustomNameOption::String(s) => return self.generate_component_name(s).to_string(),
            }
        }
        format!(
            "{}/{}/{}",
            self.options.library_name,
            self.options.library_directory,
            self.generate_component_name(source)
        )
    }

    fn generate_component_name<'a>(&self, source: &'a str) -> &'a str {
        if self.options.camel_2_dash_component_name {
            let re1 = Regex::new(r"^(?<=[a-z])([A-Z])").unwrap();
            let re2 = Regex::new(r"^[A-Z]").unwrap();
            re2.replace_all(
                &re1.replace_all(source, |caps: &Captures| format!("-{}", &caps[0])),
                |caps: &Captures| caps[0].to_lowercase(),
            );
        }
        return source;
    }
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
                    let imported =
                        if let Some(ModuleExportName::Ident(ref name)) = specifier.imported {
                            name.sym.clone()
                        } else {
                            specifier.local.sym.clone()
                        };
                    let component_path = self.generate_component_path(&imported);
                    let quote_mark = if is_double_quote(&component_path) { '"' } else { '\'' };
                    let mut new_import_item = n.clone();
                    new_import_item.specifiers = if self.options.transform_to_default_import {
                        vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                            span: specifier.span,
                            local: specifier.local.clone()
                        })]
                    } else {
                        vec![ImportSpecifier::Named(specifier.clone())]
                    };
                    new_import_item.src.value = component_path.clone().into();
                    if let Some(_) = new_import_item.src.raw {
                        new_import_item.src.raw = Some(format!("{}{}{}", quote_mark, component_path, quote_mark).into());
                    }
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
