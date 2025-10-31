use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProductOption {
    pub name: String,
    pub price: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProductItem {
    pub name: String,
    pub price: Option<f64>,
    pub options: Vec<ProductOption>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProductsCreate {
    pub items: Vec<ProductItem>,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProductsRead {
    pub items: Vec<ProductItem>,
    pub description: String,
}

impl Into<ProductsRead> for ProductsCreate {
    fn into(self) -> ProductsRead {
        ProductsRead {
            items: self.items,
            description: self.description,
        }
    }
}
