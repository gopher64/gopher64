use crate::text::TextProxy;
use async_trait::async_trait;

#[async_trait]
pub trait TextExt {
	async fn get_text_ext(&self) -> zbus::Result<String>;
}

#[async_trait]
impl TextExt for TextProxy<'_> {
	async fn get_text_ext(&self) -> zbus::Result<String> {
		let length_of_string = self.character_count().await?;
		Ok(self.get_text(0, length_of_string).await?)
	}
}
