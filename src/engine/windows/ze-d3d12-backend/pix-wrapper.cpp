#include "pix-wrapper.hpp"
#include <windows.h>
#include <dxgi1_3.h>
#include <d3d12.h>
#define USE_PIX
#include "DXProgrammableCapture.h"
#include "WinPixEventRuntime\\pix3.h"

void pix_begin_event_cmd_list(ID3D12GraphicsCommandList* command_list, uint8_t color_r, uint8_t color_g, uint8_t color_b, const wchar_t* marker)
{
    PIXBeginEvent(command_list, PIX_COLOR(color_r, color_g, color_b), marker);
}

void pix_end_event_cmd_list(ID3D12GraphicsCommandList* command_list)
{
    PIXEndEvent(command_list);
}