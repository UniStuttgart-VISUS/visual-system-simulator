#include <Windows.h>
#include <GL/GL.h>
#include <Varjo.h>
#include <Varjo_gl.h>
#include <Varjo_layers.h>
#include <Varjo_types_layers.h>
#include <cassert>
#include <stdexcept>
#include <vector>
#include <array>

struct VarjoRenderTarget
{
    GLuint colorTextureId;
    GLuint depthTextureId;
    uint32_t width;
    uint32_t height;
};

struct VarjoGazeData {
    float leftEye[3];
    float rightEye[3];
    float focusDistance;
};

class Varjo
{
public:
    varjo_Session *m_session = nullptr;
    int32_t m_viewCount;
    std::vector<varjo_Viewport> m_viewports;
    varjo_SwapChainConfig2 m_swapChainConfig;
    varjo_SwapChainConfig2 m_depthSwapChainConfig;
    varjo_SwapChain *m_colorSwapChain = nullptr;
    varjo_SwapChain *m_depthSwapChain = nullptr;
    std::vector<VarjoRenderTarget> m_renderTargets;
    int32_t m_currentSwapChainIndex = 0;
    std::vector<varjo_LayerMultiProjView> m_projectionLayers;
    std::vector<float> m_viewMatrices;
    std::vector<float> m_projMatrices;
    bool m_gazeAvailable = false;
    VarjoGazeData m_gazeData;
    varjo_FrameInfo *m_frameInfo = nullptr;
    bool m_visible = true;

    void validate()
    {
        varjo_Error error = varjo_GetError(m_session);
        if (error != varjo_NoError)
            throw std::runtime_error(varjo_GetErrorDesc(error));
    }

public:
    Varjo()
    {
        // Test if Varjo system is available.
        if (!varjo_IsAvailable())
            throw std::runtime_error("Varjo system is unavailable");

        // Create session.
        m_session = varjo_SessionInit();
        validate();

        // Enumerate and pack views into an atlas-like layout.
        m_viewCount = varjo_GetViewCount(m_session);
        m_viewports.reserve(m_viewCount);
        int x = 0, y = 0;
        for (int32_t i = 0; i < m_viewCount; i++)
        {
            const varjo_ViewDescription viewDescription = varjo_GetViewDescription(m_session, i);
            const varjo_Viewport viewport = varjo_Viewport{x, y, viewDescription.width, viewDescription.height};
            m_viewports.push_back(viewport);
            x += viewport.width;
            if (i > 0 && m_viewports.size() % 2 == 0)
            {
                x = 0;
                y += viewport.height;
            }
        }

        // Setup color swap chain (ring buffer of render targets).
        m_swapChainConfig.numberOfTextures = 4;
        m_swapChainConfig.textureArraySize = 1;
        m_swapChainConfig.textureFormat = varjo_TextureFormat_R8G8B8A8_SRGB;
        m_swapChainConfig.textureWidth = m_viewports.back().width + m_viewports.back().x;
        m_swapChainConfig.textureHeight = m_viewports.back().height + m_viewports.back().y;
        m_colorSwapChain = varjo_GLCreateSwapChain(m_session, &m_swapChainConfig);

        m_depthSwapChainConfig = m_swapChainConfig;
        m_depthSwapChainConfig.textureFormat = varjo_DepthTextureFormat_D24_UNORM_S8_UINT;
        m_depthSwapChain = varjo_GLCreateSwapChain(m_session, &m_depthSwapChainConfig);
        validate();

        // Create a render target per swap chain texture.
        for (int i = 0; i < m_swapChainConfig.numberOfTextures; ++i)
        {
            const varjo_Texture colorTexture = varjo_GetSwapChainImage(m_colorSwapChain, i);
            const varjo_Texture depthTexture = varjo_GetSwapChainImage(m_depthSwapChain, i);
            m_renderTargets.push_back(
                VarjoRenderTarget{
                    varjo_ToGLTexture(colorTexture),
                    varjo_ToGLTexture(depthTexture),
                    static_cast<uint32_t>(m_swapChainConfig.textureWidth),
                    static_cast<uint32_t>(m_swapChainConfig.textureHeight)});
        }

        // Create projection layers views
        m_projectionLayers.reserve(m_viewCount);
        m_viewMatrices.reserve(m_viewCount*16);
        m_projMatrices.reserve(m_viewCount*16);
        for (int32_t i = 0; i < m_viewCount; i++)
        {
            m_projectionLayers[i].extension = nullptr;//XXX: add usage of depth textures
            m_projectionLayers[i].viewport = varjo_SwapChainViewport{m_colorSwapChain, m_viewports[i].x, m_viewports[i].y, m_viewports[i].width, m_viewports[i].height, 0};
        }

        // Create a FrameInfo (used during main loop.)
        m_frameInfo = varjo_CreateFrameInfo(m_session);
        validate();

        // Initialize gaze tracking
        if (varjo_IsGazeAllowed(m_session)) {
            varjo_GazeInit(m_session);

            varjo_Error err = varjo_GetError(m_session);
            if (err != varjo_NoError) {
                printf("Failed to initialize Gaze: %s", varjo_GetErrorDesc(err));
            }else{
                m_gazeAvailable = true;
            }
        }else{
            printf("Gaze tracking is not allowed!\n");
        }
    }

    ~Varjo()
    {
        varjo_FreeFrameInfo(m_frameInfo);
        varjo_FreeSwapChain(m_colorSwapChain);
        varjo_FreeSwapChain(m_depthSwapChain);
        varjo_SessionShutDown(m_session);
    }

    bool beginFrameSync()
    {
        varjo_Event evt;
        while (varjo_PollEvent(m_session, &evt))
        {
            switch (evt.header.type)
            {
            case varjo_EventType_Visibility:
                m_visible = evt.data.visibility.visible;
                printf("Visible %s\n", evt.data.visibility.visible ? "true" : "false");
                break;
            case varjo_EventType_Foreground:
                printf("In foreground %s\n", evt.data.foreground.isForeground ? "true" : "false");
                break;
            case varjo_EventType_HeadsetStandbyStatus:
                printf("Headset on standby %s\n", evt.data.headsetStandbyStatus.onStandby ? "true" : "false");
                break;
            case varjo_EventType_Button:
                if (evt.data.button.buttonId == varjo_ButtonId_Application && evt.data.button.pressed)
                {
                    //gaze.requestCalibration();
                }
                break;
            case varjo_EventType_FoveationStatus:
                //renderer->useFoveatedViewports(evt.data.foveationStatus.status == varjo_FoveationStatus_Ok);
                break;
            }
        }
        if (m_visible)
        {
            // Wait before rendering the next frame.
            varjo_WaitSync(m_session, m_frameInfo);

            varjo_BeginFrameWithLayers(m_session);

            varjo_AcquireSwapChainImage(m_colorSwapChain, &m_currentSwapChainIndex);
            varjo_AcquireSwapChainImage(m_depthSwapChain, &m_currentSwapChainIndex);
            for (uint32_t i = 0; i < m_viewCount; ++i)
            {
                varjo_ViewInfo &view = m_frameInfo->views[i];
                if (!view.enabled)
                {
                    continue; // Skip a view if it is not enabled.
                }

                //...
                for(int j = 0; j < 16; ++j){
                    m_viewMatrices[i*16+j] = view.viewMatrix[j];
                    m_projMatrices[i*16+j] = view.projectionMatrix[j];
                }

                std::copy(view.projectionMatrix, view.projectionMatrix + 16, m_projectionLayers[i].projection.value);
                std::copy(view.viewMatrix, view.viewMatrix + 16, m_projectionLayers[i].view.value);
            }
        }
        return m_visible;
    }

    void endFrame()
    {
        varjo_LayerMultiProj multiProjectionLayer{
            {varjo_LayerMultiProjType, 0}, varjo_SpaceLocal, static_cast<int32_t>(m_viewCount), m_projectionLayers.data()};
        std::array<varjo_LayerHeader *, 1> layers = {&multiProjectionLayer.header};
        varjo_SubmitInfoLayers submitInfoLayers{m_frameInfo->frameNumber, 0, 1, layers.data()};

        varjo_ReleaseSwapChainImage(m_colorSwapChain);
        varjo_ReleaseSwapChainImage(m_depthSwapChain);

        varjo_EndFrameWithLayers(m_session, &submitInfoLayers);
    }
};

#define API_EXPORT extern "C"

API_EXPORT const char *varjo_new(Varjo **varjo)
{
    assert(*varjo == nullptr && "Null pointer expected");
    try
    {
        *varjo = new Varjo();
        return nullptr;
    }
    catch (const std::exception &ex)
    {
        return ex.what();
    }
    catch (...)
    {
        return "Unexpected exception";
    }
}

API_EXPORT void varjo_drop(Varjo **varjo)
{
    delete *varjo;
    *varjo = nullptr;
}

API_EXPORT const char *varjo_render_targets(Varjo *varjo, VarjoRenderTarget **render_targets, uint32_t *render_target_size)
{
    assert(varjo != nullptr && "Varjo instance expected");
    try
    {
        *render_targets = varjo->m_renderTargets.data();
        *render_target_size = static_cast<uint32_t>(varjo->m_renderTargets.size());
        return nullptr;
    }
    catch (const std::exception &ex)
    {
        return ex.what();
    }
    catch (...)
    {
        return "Unexpected exception";
    }
}

API_EXPORT const char *varjo_begin_frame_sync(Varjo *varjo, bool *is_available)
{
    assert(varjo != nullptr && "Varjo instance expected");
    try
    {
        *is_available = varjo->beginFrameSync();
        return nullptr;
    }
    catch (const std::exception &ex)
    {
        return ex.what();
    }
    catch (...)
    {
        return "Unexpected exception";
    }
}

API_EXPORT const char *varjo_current_swap_chain_index(Varjo *varjo, uint32_t *current_swap_chain_index)
{
    assert(varjo != nullptr && "Varjo instance expected");
    try
    {
        *current_swap_chain_index = static_cast<uint32_t>(varjo->m_currentSwapChainIndex);
        return nullptr;
    }
    catch (const std::exception &ex)
    {
        return ex.what();
    }
    catch (...)
    {
        return "Unexpected exception";
    }
}

API_EXPORT const char *varjo_current_view_matrices(Varjo *varjo, float **view_matrix_values, uint32_t *view_matrix_count)
{
    assert(varjo != nullptr && "Varjo instance expected");
    try
    {
        *view_matrix_values = varjo->m_viewMatrices.data();
        *view_matrix_count = static_cast<uint32_t>(varjo->m_viewCount);
        return nullptr;
    }
    catch (const std::exception &ex)
    {
        return ex.what();
    }
    catch (...)
    {
        return "Unexpected exception";
    }
}

API_EXPORT const char *varjo_current_proj_matrices(Varjo *varjo, float **proj_matrix_values, uint32_t *proj_matrix_count)
{
    assert(varjo != nullptr && "Varjo instance expected");
    try
    {
        *proj_matrix_values = varjo->m_projMatrices.data();
        *proj_matrix_count = static_cast<uint32_t>(varjo->m_viewCount);
        return nullptr;
    }
    catch (const std::exception &ex)
    {
        return ex.what();
    }
    catch (...)
    {
        return "Unexpected exception";
    }
}

API_EXPORT const char *varjo_current_gaze_data(Varjo *varjo, bool *is_valid, VarjoGazeData *gaze_data) //TODO left, right eye, plus filtering with status, plus focus dist.
{
    assert(varjo != nullptr && "Varjo instance expected");
    try
    {
        *is_valid = false;

        if (!varjo->m_gazeAvailable) return nullptr;
        
        varjo_SyncProperties(varjo->m_session);

        // Get gaze and check that it is valid
        varjo_Gaze gaze = varjo_GetGaze(varjo->m_session);
        if (gaze.status == varjo_GazeStatus_Invalid) return nullptr;

        varjo->m_gazeData.leftEye[0] = gaze.leftEye.forward[0];
        varjo->m_gazeData.leftEye[1] = gaze.leftEye.forward[1];
        varjo->m_gazeData.leftEye[2] = gaze.leftEye.forward[2];
        varjo->m_gazeData.rightEye[0] = gaze.rightEye.forward[0];
        varjo->m_gazeData.rightEye[1] = gaze.rightEye.forward[1];
        varjo->m_gazeData.rightEye[2] = gaze.rightEye.forward[2];
        varjo->m_gazeData.focusDistance = gaze.focusDistance;
        *gaze_data = varjo->m_gazeData;

        *is_valid = true;
        return nullptr;
    }
    catch (const std::exception &ex)
    {
        return ex.what();
    }
    catch (...)
    {
        return "Unexpected exception";
    }
}

API_EXPORT const char *varjo_end_frame(Varjo *varjo)
{
    assert(varjo != nullptr && "Varjo instance expected");
    try
    {
        varjo->endFrame();
        return nullptr;
    }
    catch (const std::exception &ex)
    {
        return ex.what();
    }
    catch (...)
    {
        return "Unexpected exception";
    }
}
