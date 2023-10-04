#include "wsi_platform.hpp"
#include <SDL_vulkan.h>

VkSurfaceKHR SDL_WSIPlatform::create_surface(VkInstance instance, VkPhysicalDevice gpu)
{
	VkSurfaceKHR surface = nullptr;
	SDL_bool result = SDL_Vulkan_CreateSurface(window, instance, &surface);
	if (result != SDL_TRUE)
	{
		printf("Error creating surface\n");
	}
	return surface;
}

void SDL_WSIPlatform::destroy_surface(VkInstance instance, VkSurfaceKHR surface)
{
}

std::vector<const char *> SDL_WSIPlatform::get_instance_extensions()
{

	unsigned int extensionCount = 0;
	SDL_Vulkan_GetInstanceExtensions(window, &extensionCount, nullptr);
	std::vector<const char *> extensionNames(extensionCount);
	SDL_bool result = SDL_Vulkan_GetInstanceExtensions(window, &extensionCount, extensionNames.data());
	if (result != SDL_TRUE)
	{
		printf("Error creating surface\n");
	}
	return extensionNames;
}

uint32_t SDL_WSIPlatform::get_surface_width()
{
	int w, h;
	SDL_GetWindowSize(window, &w, &h);
	return w;
}

uint32_t SDL_WSIPlatform::get_surface_height()
{
	int w, h;
	SDL_GetWindowSize(window, &w, &h);
	return h;
}

bool SDL_WSIPlatform::alive(Vulkan::WSI &wsi)
{
	return true;
}

void SDL_WSIPlatform::poll_input()
{
	SDL_PumpEvents();
}

void SDL_WSIPlatform::set_window(SDL_Window *_window)
{
	window = _window;
}
