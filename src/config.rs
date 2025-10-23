use serde::{Deserialize, Serialize};
use serde_json::Value;
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::ModuleItem;
use swc_ecma_parser::{Lexer, Parser, StringInput, Syntax, TsSyntax};
use tokio::fs::read_to_string;

use crate::{error::Error, utils::expr_to_value::expr_to_value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub languages: LanguageNode,
    pub targets: Vec<Target>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            languages: LanguageNode {
                name: "en".to_string(),
                children: vec![],
            },
            targets: vec![Target {
                includes: vec!["src".to_string()],
                excludes: vec!["node_modules".to_string(), ".*".to_string()],
                output: default_output(),
                fn_names: default_fn_names(),
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageNode {
    pub name: String,
    #[serde(default)]
    pub children: Vec<LanguageNode>,
}

impl LanguageNode {
    pub fn collect_languages(&self) -> Vec<String> {
        let mut result = Vec::new();
        self.collect_recursive(&mut result);
        result
    }

    fn collect_recursive(&self, result: &mut Vec<String>) {
        result.push(self.name.clone());
        for child in &self.children {
            child.collect_recursive(result);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    #[serde(default = "default_output")]
    pub output: String,
    #[serde(default = "default_fn_names")]
    pub fn_names: Vec<String>,
}

fn default_output() -> String {
    "_t".to_string()
}

fn default_fn_names() -> Vec<String> {
    vec!["t".to_string()]
}

pub async fn load_config_from_file(path: &str) -> Result<Config, Error> {
    let content = read_to_string(path).await?;

    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Real(path.into()).into(), content.clone());
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: true,
            decorators: true,
            ..Default::default()
        }),
        swc_ecma_ast::EsVersion::EsNext,
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module().map_err(Error::ParseModule)?;

    // 1. Collect all variable definitions
    let mut var_map = std::collections::HashMap::new();
    for item in &module.body {
        if let swc_ecma_ast::ModuleItem::Stmt(swc_ecma_ast::Stmt::Decl(swc_ecma_ast::Decl::Var(
            var,
        ))) = item
        {
            for decl in &var.decls {
                if let Some(init) = &decl.init {
                    if let swc_ecma_ast::Pat::Ident(ident) = &decl.name {
                        var_map.insert(ident.id.sym.to_string(), init.as_ref());
                    }
                }
            }
        }
    }

    // 2. Find export default and parse the object it points to
    for item in &module.body {
        if let ModuleItem::ModuleDecl(swc_ecma_ast::ModuleDecl::ExportDefaultExpr(expr)) = item {
            let exported_expr = &expr.expr;
            let target_expr = match &**exported_expr {
                swc_ecma_ast::Expr::Ident(ident) => {
                    var_map.get(&ident.sym.to_string()).ok_or_else(|| {
                        Error::Error(format!(
                            "Identifier {} not found in variable map",
                            ident.sym
                        ))
                    })?
                }
                _ => &**exported_expr,
            };

            let value: Value = expr_to_value(target_expr)?;
            return Ok(serde_json::from_value(value)?);
        }
    }

    Err(Error::Error("No exported expression found".to_string()))
}
