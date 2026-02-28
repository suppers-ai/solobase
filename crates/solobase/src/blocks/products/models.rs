use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: String,
    pub group_id: Option<String>,
    pub product_type: String,
    pub status: String,
    pub pricing_template_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductGroup {
    pub id: String,
    pub name: String,
    pub description: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingTemplate {
    pub id: String,
    pub name: String,
    pub formula: String,
    pub variables: Vec<String>,
    pub conditions: Option<Vec<PricingCondition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingCondition {
    pub field: String,
    pub operator: String,
    pub value: serde_json::Value,
    pub formula: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Purchase {
    pub id: String,
    pub user_id: String,
    pub status: String,
    pub total_amount: f64,
    pub currency: String,
    pub payment_provider: String,
    pub payment_id: Option<String>,
    pub line_items: Vec<LineItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    pub product_id: String,
    pub product_name: String,
    pub quantity: i64,
    pub unit_price: f64,
    pub total_price: f64,
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub id: String,
    pub name: String,
    pub var_type: String,
    pub default_value: Option<serde_json::Value>,
    pub scope: String,
    pub product_id: Option<String>,
}
