use std::collections::HashMap;

use wafer_core::clients::database as db;
use wafer_run::{context::Context, InputStream, OutputStream};

use super::PRODUCTS_TABLE;
use crate::blocks::helpers::{
    err_bad_request, err_internal_no_cause, err_not_found, ok_json, RecordExt,
};

/// Pricing template table — reusable pricing rule definitions (formulas,
/// tiers, etc.) referenced by products at calc time.
pub(crate) const TABLE: &str = "suppers_ai__products__pricing_templates";

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
    let product = match db::get(ctx, PRODUCTS_TABLE, &body.product_id).await {
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
        if let Err(e) = validate_price(base_price) {
            return err_bad_request(&e);
        }
        let total = base_price * body.quantity as f64;
        return ok_json(&serde_json::json!({
            "unit_price": base_price,
            "quantity": body.quantity,
            "total": total,
            "currency": product.data.get("currency").and_then(|v| v.as_str()).unwrap_or("USD")
        }));
    }

    let template = match db::get(ctx, TABLE, template_id).await {
        Ok(t) => t,
        Err(_) => return err_internal_no_cause("Pricing template not found"),
    };

    let formula = template.str_field("price_formula");
    if formula.is_empty() {
        return err_internal_no_cause("Empty pricing formula");
    }

    // Evaluate formula
    let unit_price = match evaluate_formula(formula, &body.variables) {
        Ok(p) => p,
        Err(e) => return err_bad_request(&format!("Formula evaluation error: {e}")),
    };
    if let Err(e) = validate_price(unit_price) {
        return err_bad_request(&e);
    }

    let total = unit_price * body.quantity as f64;

    ok_json(&serde_json::json!({
        "unit_price": unit_price,
        "quantity": body.quantity,
        "total": total,
        "currency": product.data.get("currency").and_then(|v| v.as_str()).unwrap_or("USD"),
        "formula": formula,
        "variables_used": body.variables
    }))
}

/// Minimum acceptable price for a product (in display currency units).
/// Anything below this is treated as an attempt at price manipulation or a
/// formula bug, not a legitimate sale.
pub const MIN_PRICE: f64 = 0.01;

/// Validate an evaluated unit/final price.
///
/// Rejects NaN, non-finite values, and any price <= 0.0. Enforces a minimum
/// of `MIN_PRICE` (1 cent). Returns a human-readable error suitable for
/// `err_bad_request`.
pub fn validate_price(price: f64) -> Result<(), String> {
    if !price.is_finite() {
        return Err("Invalid price: not a finite number".to_string());
    }
    if price <= 0.0 {
        return Err("Invalid price: must be greater than zero".to_string());
    }
    if price < MIN_PRICE {
        return Err(format!(
            "Invalid price: must be at least {MIN_PRICE} (got {price})"
        ));
    }
    Ok(())
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
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' => {
                chars.next();
            }
            '+' => {
                chars.next();
                tokens.push(Token::Plus);
            }
            '-' => {
                chars.next();
                tokens.push(Token::Minus);
            }
            '*' => {
                chars.next();
                tokens.push(Token::Star);
            }
            '/' => {
                chars.next();
                tokens.push(Token::Slash);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            c if c.is_ascii_digit() || c == '.' => {
                let mut num_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        num_str.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let num = num_str
                    .parse::<f64>()
                    .map_err(|e| format!("Invalid number: {e}"))?;
                tokens.push(Token::Number(num));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
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
