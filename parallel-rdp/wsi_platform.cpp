#include "wsi_platform.hpp"
#include <SDL3/SDL_vulkan.h>

VkSurfaceKHR SDL_WSIPlatform::create_surface(VkInstance instance, VkPhysicalDevice gpu)
{
	VkSurfaceKHR surface = nullptr;
	bool result = SDL_Vulkan_CreateSurface(window, instance, NULL, &surface);
	if (result != true)
	{
		printf("Error creating surface\n");
	}
	return surface;
}

void SDL_WSIPlatform::destroy_surface(VkInstance instance, VkSurfaceKHR surface)
{
	SDL_Vulkan_DestroySurface(instance, surface, NULL);
}

std::vector<const char *> SDL_WSIPlatform::get_instance_extensions()
{

	unsigned int extensionCount = 0;
	char const *const *extensions = SDL_Vulkan_GetInstanceExtensions(&extensionCount);
	if (extensions == NULL)
	{
		printf("Error getting instance extensions\n");
	}

	std::vector<const char *> extensionNames;
	for (unsigned int i = 0; i < extensionCount; ++i)
	{
		extensionNames.push_back(extensions[i]);
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
}

void SDL_WSIPlatform::poll_input_async(Granite::InputTrackerHandler *handler)
{
}

void SDL_WSIPlatform::set_window(SDL_Window *_window)
{
	window = _window;
}

void SDL_WSIPlatform::do_resize()
{
	resize = true;
}
