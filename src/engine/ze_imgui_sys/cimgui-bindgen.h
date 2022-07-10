#define CIMGUI_DEFINE_ENUMS_AND_STRUCTS

#include "third-party/cimgui/cimgui.h"

CIMGUI_API void ImGuiPlatformIO_Set_Platform_GetWindowPos(ImGuiPlatformIO* platform_io, void(*user_callback)(ImGuiViewport* vp, ImVec2* out_pos));
CIMGUI_API void ImGuiPlatformIO_Set_Platform_GetWindowSize(ImGuiPlatformIO* platform_io, void(*user_callback)(ImGuiViewport* vp, ImVec2* out_size));
