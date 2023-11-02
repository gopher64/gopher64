use async_trait::async_trait;
use crate::hyperlink::Hyperlink;

#[async_trait]
pub trait HyperlinkPlus {
}

#[async_trait]
impl HyperlinkPlus for Hyperlink {
}
