use serde::Deserialize;

pub(crate) const PRODUCTS_JSON: &str = include_str!("../../web/src/catalog.json");

#[derive(Deserialize)]
pub(crate) struct ProductCatalog {
    products: Vec<CatalogProduct>,
}

impl ProductCatalog {
    #[must_use]
    pub(crate) fn product(&self, product_id: &str) -> Option<&CatalogProduct> {
        self.products
            .iter()
            .find(|product| product.id == product_id)
    }
}

#[derive(Deserialize)]
pub(crate) struct CatalogProduct {
    id: String,
    pub(crate) name: String,
    #[serde(rename = "priceCents")]
    pub(crate) price_cents: u64,
}

pub(crate) fn product_catalog() -> Result<ProductCatalog, String> {
    serde_json::from_str::<ProductCatalog>(PRODUCTS_JSON)
        .map_err(|_error| "product catalog is unavailable".to_owned())
}
