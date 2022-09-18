#include <cstdint>

typedef struct ID3D12GraphicsCommandList;
typedef struct ID3D12CommandQueue;

#ifdef __cplusplus
extern "C"
{
#endif
    void pix_begin_event_cmd_list(ID3D12GraphicsCommandList* command_list, uint8_t color_r, uint8_t color_g, uint8_t color_b, const wchar_t *marker);
    void pix_end_event_cmd_list(ID3D12GraphicsCommandList* command_list);
#ifdef __cplusplus
}
#endif