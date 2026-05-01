use crate::models::casebuilder::WorkProduct;
use crate::services::html_renderer::work_product_plain_text;

pub(crate) fn render_docx_placeholder(product: &WorkProduct) -> String {
    format!(
        "{}\n\nDOCX renderer placeholder. Review needed.\n\n{}",
        product.title,
        work_product_plain_text(product)
    )
}
