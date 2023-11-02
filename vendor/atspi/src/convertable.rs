use crate::{
	accessible::AccessibleProxy, action::ActionProxy, application::ApplicationProxy,
	cache::CacheProxy, collection::CollectionProxy, component::ComponentProxy,
	device_event_controller::DeviceEventControllerProxy,
	device_event_listener::DeviceEventListenerProxy, document::DocumentProxy,
	editable_text::EditableTextProxy, hyperlink::HyperlinkProxy, hypertext::HypertextProxy,
	image::ImageProxy, registry::RegistryProxy, selection::SelectionProxy, table::TableProxy,
	table_cell::TableCellProxy, text::TextProxy, value::ValueProxy, AtspiProxy,
};
use async_trait::async_trait;
use std::ops::Deref;
use zbus::{CacheProperties, Error, Proxy, ProxyBuilder, ProxyDefault};

#[async_trait]
pub trait Convertable {
	async fn to_accessible(&self) -> zbus::Result<AccessibleProxy<'_>>;
	async fn to_action(&self) -> zbus::Result<ActionProxy<'_>>;
	async fn to_application(&self) -> zbus::Result<ApplicationProxy<'_>>;
	async fn to_collection(&self) -> zbus::Result<CollectionProxy<'_>>;
	async fn to_component(&self) -> zbus::Result<ComponentProxy<'_>>;
	async fn to_document(&self) -> zbus::Result<DocumentProxy<'_>>;
	async fn to_hypertext(&self) -> zbus::Result<HypertextProxy<'_>>;
	async fn to_hyperlink(&self) -> zbus::Result<HyperlinkProxy<'_>>;
	async fn to_image(&self) -> zbus::Result<ImageProxy<'_>>;
	async fn to_selection(&self) -> zbus::Result<SelectionProxy<'_>>;
	async fn to_table(&self) -> zbus::Result<TableProxy<'_>>;
	async fn to_table_cell(&self) -> zbus::Result<TableCellProxy<'_>>;
	async fn to_text(&self) -> zbus::Result<TextProxy<'_>>;
	async fn to_editable_text(&self) -> zbus::Result<EditableTextProxy<'_>>;
	async fn to_cache(&self) -> zbus::Result<CacheProxy<'_>>;
	async fn to_value(&self) -> zbus::Result<ValueProxy<'_>>;
	async fn to_registry(&self) -> zbus::Result<RegistryProxy<'_>>;
	async fn to_device_event_controller(&self) -> zbus::Result<DeviceEventControllerProxy<'_>>;
	async fn to_device_event_listener(&self) -> zbus::Result<DeviceEventListenerProxy<'_>>;
}

#[inline]
async fn convert_to_new_type<
	'a,
	'b,
	T: From<Proxy<'b>> + ProxyDefault,
	U: Deref<Target = Proxy<'a>> + ProxyDefault + AtspiProxy,
>(
	from: &U,
) -> zbus::Result<T> {
	// first thing is first, we need to creat an accessible to query the interfaces.
	let accessible = AccessibleProxy::builder(from.connection())
		.destination(from.destination())?
		.cache_properties(CacheProperties::No)
		.path(from.path())?
		.build()
		.await?;
	// if the interface we're trying to convert to is not available as an interface; this can be problematic because the interface we're passing in could potentially be different from what we're converting to.
	if !accessible
		.get_interfaces()
		.await?
		.contains(<U as AtspiProxy>::INTERFACE)
	{
		return Err(Error::InterfaceNotFound);
	}
	// otherwise, make a new Proxy with the related type.
	let path = from.path().to_owned();
	let dest = from.destination().to_owned();
	ProxyBuilder::<'b, T>::new_bare(from.connection())
		.interface(<T as ProxyDefault>::INTERFACE)?
		.destination(dest)?
		.cache_properties(CacheProperties::No)
		.path(path)?
		.build()
		.await
}

#[async_trait]
impl<'a, T: Deref<Target = Proxy<'a>> + ProxyDefault + AtspiProxy + Sync> Convertable for T {
	/* no guard due to assumption it is always possible */
	async fn to_accessible(&self) -> zbus::Result<AccessibleProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_action(&self) -> zbus::Result<ActionProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_application(&self) -> zbus::Result<ApplicationProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_collection(&self) -> zbus::Result<CollectionProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_component(&self) -> zbus::Result<ComponentProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_document(&self) -> zbus::Result<DocumentProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_hypertext(&self) -> zbus::Result<HypertextProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_hyperlink(&self) -> zbus::Result<HyperlinkProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_image(&self) -> zbus::Result<ImageProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_selection(&self) -> zbus::Result<SelectionProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_table(&self) -> zbus::Result<TableProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_table_cell(&self) -> zbus::Result<TableCellProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_text(&self) -> zbus::Result<TextProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_editable_text(&self) -> zbus::Result<EditableTextProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_cache(&self) -> zbus::Result<CacheProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_value(&self) -> zbus::Result<ValueProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_registry(&self) -> zbus::Result<RegistryProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_device_event_controller(&self) -> zbus::Result<DeviceEventControllerProxy<'_>> {
		convert_to_new_type(self).await
	}
	async fn to_device_event_listener(&self) -> zbus::Result<DeviceEventListenerProxy<'_>> {
		convert_to_new_type(self).await
	}
}
