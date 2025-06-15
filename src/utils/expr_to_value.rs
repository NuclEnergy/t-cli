use serde_json::Value;
use swc_ecma_ast::{Expr, Lit, Prop, PropName, PropOrSpread};

use crate::error::Error;

pub fn expr_to_value(expr: &Expr) -> Result<Value, Error> {
    match &expr {
        Expr::TsSatisfies(e) => expr_to_value(&e.expr),
        Expr::TsConstAssertion(e) => expr_to_value(&e.expr),
        Expr::Object(obj) => {
            let mut map = serde_json::Map::new();
            for prop in &obj.props {
                if let PropOrSpread::Prop(prop_box) = prop {
                    if let Prop::KeyValue(kv) = &**prop_box {
                        let key = match &kv.key {
                            PropName::Ident(ident) => ident.sym.to_string(),
                            PropName::Str(s) => s.value.to_string(),
                            _ => return Err(Error::Error(format!("Invalid key: {:?}", kv.key))),
                        };
                        let value = expr_to_value(&kv.value)?;
                        map.insert(key, value);
                    }
                }
            }
            Ok(Value::Object(map))
        }
        Expr::Array(arr) => {
            let mut vec = Vec::new();
            for elem in &arr.elems {
                if let Some(e) = elem {
                    vec.push(expr_to_value(&e.expr)?);
                }
            }
            Ok(Value::Array(vec))
        }
        Expr::Lit(Lit::Str(s)) => Ok(Value::String(s.value.to_string())),
        _ => Err(Error::Error(format!(
            "Unsuported expression type: {:?}",
            expr
        ))),
    }
}
