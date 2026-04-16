use std::collections::HashMap;

use wafer_core::clients::database as db;
use wafer_run::{context::Context, InputStream, OutputStream};

use super::{PRICING_COLLECTION, PRODUCTS_COLLECTION};
use crate::blocks::helpers::{err_bad_request, err_internal, err_not_found, ok_json, RecordExt};

pub async fn handle_calculate(ctx: &dyn Context, input: InputStream) -> OutputStream {
    #[derive(serde::Deserialize)]
    struct CalcReq {
        product_id: String,
        #[serde(default)]
        variables: HashMap<String, f64>,
        #[serde(default = "default_quantity")]
        quantity: i64,
    }
    fn default_quantity() -> i64 {
        1
    }

    let raw = input.collect_to_bytes().await;
    let body: CalcReq = match serde_json::from_slice(&raw) {
        Ok(b) => b,
        Err(e) => return err_bad_request(&format!("Invalid body: {e}")),
    };

    // Get product
    let product = match db::get(ctx, PRODUCTS_COLLECTION, &body.product_id).await {
        Ok(p) => p,
        Err(_) => return err_not_found("Product not found"),
    };

    // Get pricing template
    let template_id = product.str_field("pricing_template_id");
    if template_id.is_empty() {
        // Direct price from product
        let base_price = product
            .data
            .get("base_price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let total = base_price * body.quantity as f64;
        return ok_json(&serde_json::json!({
            "unit_price": base_price,
            "quantity": body.quantity,
            "total": total,
            "currency": product.data.get("currency").and_then(|v| v.as_str()).unwrap_or("USD")
        }));
    }

    let template = match db::get(ctx, PRICING_COLLECTION, template_id).await {
        Ok(t) => t,
        Err(_) => return err_internal("Pricing template not found"),
    };

    let formula = template.str_field("price_formula");
    if formula.is_empty() {
        return err_internal("Empty pricing formula");
    }

    // Evaluate formula
    let unit_price = match evaluate_formula(formula, &body.variables) {
        Ok(p) => p,
        Err(e) => return err_bad_request(&format!("Formula evaluation error: {e}")),
    };

    // Check conditions
    let conditions = template.data.get("conditions");
    let final_price = if let Some(serde_json::Value::Array(conds)) = conditions {
        let mut price = unit_price;
        for cond in conds {
            if let Some(cond_obj) = cond.as_object() {
                if evaluate_condition(cond_obj, &body.variables) {
                    if let Some(cond_formula) = cond_obj.get("formula").and_then(|v| v.as_str()) {
                        if let Ok(p) = evaluate_formula(cond_formula, &body.variables) {
                            price = p;
                        }
                    }
                }
            }
        }
        price
    } else {
        unit_price
    };

    let total = final_price * body.quantity as f64;

    ok_json(&serde_json::json!({
        "unit_price": final_price,
        "quantity": body.quantity,
        "total": total,
        "currency": product.data.get("currency").and_then(|v| v.as_str()).unwrap_or("USD"),
        "formula": formula,
        "variables_used": body.variables
    }))
}

/// Evaluate a simple pricing formula.
/// Supports: numbers, +, -, *, /, parentheses, and variable references.
/// Variables are referenced by name: `base_price * quantity + shipping`
pub fn evaluate_formula(formula: &str, variables: &HashMap<String, f64>) -> Result<f64, String> {
    let tokens = tokenize(formula)?;
    let mut pos = 0;
    let result = parse_expression(&tokens, &mut pos, variables)?;
    Ok(result)
}

#[derive(Debug, Clone)]
enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' => i += 1,
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num_str: String = chars[start..i].iter().collect();
                let num = num_str
                    .parse::<f64>()
                    .map_err(|e| format!("Invalid number: {e}"))?;
                tokens.push(Token::Number(num));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect();
                tokens.push(Token::Ident(ident));
            }
            c => return Err(format!("Unexpected character: {c}")),
        }
    }
    Ok(tokens)
}

fn parse_expression(
    tokens: &[Token],
    pos: &mut usize,
    vars: &HashMap<String, f64>,
) -> Result<f64, String> {
    let mut left = parse_term(tokens, pos, vars)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Plus => {
                *pos += 1;
                left += parse_term(tokens, pos, vars)?;
            }
            Token::Minus => {
                *pos += 1;
                left -= parse_term(tokens, pos, vars)?;
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_term(
    tokens: &[Token],
    pos: &mut usize,
    vars: &HashMap<String, f64>,
) -> Result<f64, String> {
    let mut left = parse_factor(tokens, pos, vars)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Star => {
                *pos += 1;
                left *= parse_factor(tokens, pos, vars)?;
            }
            Token::Slash => {
                *pos += 1;
                let right = parse_factor(tokens, pos, vars)?;
                if right == 0.0 {
                    return Err("Division by zero".to_string());
                }
                left /= right;
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_factor(
    tokens: &[Token],
    pos: &mut usize,
    vars: &HashMap<String, f64>,
) -> Result<f64, String> {
    if *pos >= tokens.len() {
        return Err("Unexpected end of expression".to_string());
    }
    match &tokens[*pos] {
        Token::Number(n) => {
            let v = *n;
            *pos += 1;
            Ok(v)
        }
        Token::Ident(name) => {
            *pos += 1;
            vars.get(name)
                .copied()
                .ok_or_else(|| format!("Unknown variable: {name}"))
        }
        Token::LParen => {
            *pos += 1;
            let val = parse_expression(tokens, pos, vars)?;
            if *pos < tokens.len() {
                if let Token::RParen = &tokens[*pos] {
                    *pos += 1;
                } else {
                    return Err("Expected closing parenthesis".to_string());
                }
            } else {
                return Err("Missing closing parenthesis".to_string());
            }
            Ok(val)
        }
        Token::Minus => {
            *pos += 1;
            let val = parse_factor(tokens, pos, vars)?;
            Ok(-val)
        }
        _ => Err(format!("Unexpected token at position {}", pos)),
    }
}

fn evaluate_condition(
    cond: &serde_json::Map<String, serde_json::Value>,
    variables: &HashMap<String, f64>,
) -> bool {
    let field = cond.get("field").and_then(|v| v.as_str()).unwrap_or("");
    let operator = cond.get("operator").and_then(|v| v.as_str()).unwrap_or("");
    let value = cond.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let field_value = variables.get(field).copied().unwrap_or(0.0);

    match operator {
        ">" | "gt" => field_value > value,
        ">=" | "gte" => field_value >= value,
        "<" | "lt" => field_value < value,
        "<=" | "lte" => field_value <= value,
        "==" | "eq" => (field_value - value).abs() < f64::EPSILON,
        "!=" | "neq" => (field_value - value).abs() >= f64::EPSILON,
        _ => false,
    }
}
