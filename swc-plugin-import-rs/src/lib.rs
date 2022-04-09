use regex::{Captures, Regex};
use serde::Deserialize;
use swc_plugin::{ast::*, plugin_transform, TransformPluginProgramMetadata};

#[derive(Deserialize)]
#[serde(untagged)]
enum CustomNameOption {
    String(String),
}

#[derive(Deserialize, PartialEq)]
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

fn wrap_str(s: &str, wrap_char: char) -> String {
    return format!("{}{}{}", wrap_char, s, wrap_char);
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

    fn generate_style_source<'a>(&self, source: &'a str) -> String {
        if let Some(style_library_directory) = &self.options.style_library_directory {
            format!(
                "{}/{}/{}",
                self.options.library_name,
                style_library_directory,
                self.generate_component_name(source)
            )
        } else {
            let s = if let StyleOption::Bool(true) = self.options.style {
                "style"
            } else {
                "style/css"
            };
            format!("{}/{}", self.generate_component_path(source), s)
        }
    }

    fn should_import_style(&self) -> bool {
        return self.options.style != StyleOption::Bool(false)
            || self.options.style_library_directory.is_some();
    }

    fn _visit_mut_stat(&mut self, s: &mut Stmt) -> Vec<Stmt> {
        let mut ret = vec![];
        if let Stmt::Decl(Decl::Var(decl)) = s {
            for i in (0..decl.decls.len()).rev() {
                let declaration = &mut decl.decls[i];
                if let Some(Expr::Call(expr)) = declaration.init.as_deref() {
                    if let Callee::Expr(callee) = &expr.callee {
                        if let Expr::Ident(v) = callee.as_ref() {
                            if &v.sym == "require" {
                                if !expr.args.is_empty() {
                                    if let Expr::Lit(Lit::Str(v)) = expr.args[0].expr.as_ref() {
                                        if v.value == self.options.library_name {
                                            if let Pat::Object(obj_pat) = &mut declaration.name {
                                                let properties = &mut obj_pat.props;
                                                for i in (0..properties.len()).rev() {
                                                    let property = properties[i].clone();
                                                    let mut value: Option<Pat> = None;
                                                    let mut key: Option<Ident> = None;
                                                    if let ObjectPatProp::KeyValue(kv) = &property {
                                                        if let PropName::Ident(ident) = &kv.key {
                                                            key = Some(ident.clone());
                                                            value = Some(*kv.value.clone());
                                                        }
                                                    } else if let ObjectPatProp::Assign(assign) =
                                                        &property
                                                    {
                                                        key = Some(assign.key.clone());
                                                        value = Some(Pat::Ident(BindingIdent {
                                                            id: assign.key.clone(),
                                                            type_ann: None,
                                                        }));
                                                    }

                                                    if let Some(k) = &key {
                                                        if let Some(val) = &value {
                                                            let component_path = self
                                                                .generate_component_path(&k.sym);
                                                            let quote_mark = if v.raw.is_some() {
                                                                if is_double_quote(
                                                                    v.raw.as_ref().unwrap(),
                                                                ) {
                                                                    '"'
                                                                } else {
                                                                    '\''
                                                                }
                                                            } else {
                                                                '\''
                                                            };

                                                            let mut new_decl = VarDecl {
                                                                span: decl.span,
                                                                kind: decl.kind,
                                                                declare: decl.declare,
                                                                decls: vec![VarDeclarator {
                                                                    span: declaration.span,
                                                                    definite: false,
                                                                    name: if self
                                                                        .options
                                                                        .transform_to_default_import
                                                                    {
                                                                        val.clone()
                                                                    } else {
                                                                        Pat::Object(ObjectPat {
                                                                            span: k.span,
                                                                            props: vec![
                                                                                property.clone()
                                                                            ],
                                                                            optional: false,
                                                                            type_ann: None,
                                                                        })
                                                                    },
                                                                    init: None,
                                                                }],
                                                            };

                                                            if let Expr::Lit(Lit::Str(s)) =
                                                                expr.args[0].expr.as_ref()
                                                            {
                                                                let mut new_init = expr.clone();
                                                                let mut new_lit = s.clone();
                                                                new_lit.value =
                                                                    component_path.clone().into();
                                                                new_lit.raw = Some(
                                                                    wrap_str(
                                                                        &component_path,
                                                                        quote_mark,
                                                                    )
                                                                    .into(),
                                                                );
                                                                new_init.args =
                                                                    vec![ExprOrSpread {
                                                                        spread: None,
                                                                        expr: Box::new(Expr::Lit(
                                                                            Lit::Str(new_lit),
                                                                        )),
                                                                    }];
                                                                new_decl.decls[0].init = Some(
                                                                    Box::new(Expr::Call(new_init)),
                                                                );
                                                            }
                                                            ret.push(Stmt::Decl(Decl::Var(
                                                                new_decl,
                                                            )));
                                                            if self.should_import_style() {
                                                                let style_path = self
                                                                    .generate_style_source(&k.sym);

                                                                ret.push(Stmt::Expr(ExprStmt {
                                                                    span: decl.span,
                                                                    expr: Box::new(Expr::Call(CallExpr {
                                                                        span: expr.span,
                                                                        callee: expr.callee.clone(),
                                                                        type_args: None,
                                                                        args: vec![ExprOrSpread {
                                                                            spread: None,
                                                                            expr: Box::new(Expr::Lit(Lit::Str(Str {
                                                                                span: v.span,
                                                                                value: style_path.clone().into(),
                                                                                raw: Some(wrap_str(&style_path, quote_mark).into())
                                                                            })))
                                                                        }]
                                                                    }))
                                                                }))
                                                            }
                                                            properties.remove(i);
                                                        }
                                                    }
                                                }

                                                if let Pat::Object(v) = &declaration.name {
                                                    if v.props.is_empty() {
                                                        decl.decls.remove(i);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if decl.decls.is_empty() {
                *s = Stmt::Empty(EmptyStmt {
                    span: decl.span
                })
            }
        }
        ret
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
                if let ImportSpecifier::Named(specifier) = &n.specifiers[i] {
                    let imported =
                        if let Some(ModuleExportName::Ident(ref name)) = specifier.imported {
                            name.sym.clone()
                        } else {
                            specifier.local.sym.clone()
                        };
                    let component_path = self.generate_component_path(&imported);
                    let quote_mark = if is_double_quote(&component_path) {
                        '"'
                    } else {
                        '\''
                    };
                    let mut new_import_item = n.clone();
                    new_import_item.specifiers = if self.options.transform_to_default_import {
                        vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                            span: specifier.span,
                            local: specifier.local.clone(),
                        })]
                    } else {
                        vec![ImportSpecifier::Named(specifier.clone())]
                    };
                    new_import_item.src.value = component_path.clone().into();
                    if let Some(_) = new_import_item.src.raw {
                        new_import_item.src.raw =
                            Some(wrap_str(&component_path, quote_mark).into());
                    }

                    if self.should_import_style() {
                        let style_path = self.generate_style_source(&imported);
                        self.import_items.push(ImportDecl {
                            span: specifier.span,
                            specifiers: vec![],
                            type_only: false,
                            asserts: None,
                            src: Str {
                                span: n.src.span,
                                value: style_path.clone().into(),
                                raw: Some(wrap_str(&style_path, quote_mark).into()),
                            },
                        });
                    }
                    n.specifiers.remove(i);
                }
            }
        }
    }

    fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
        for i in (0..stmts.len()).rev() {
            let mut items = self._visit_mut_stat(&mut stmts[i]);
            if let Stmt::Empty(_) = &stmts[i] {
                stmts.remove(i);
            }
            if i < stmts.len() {
                stmts.splice(i..i, items);
            } else {
                stmts.append(&mut items);
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
