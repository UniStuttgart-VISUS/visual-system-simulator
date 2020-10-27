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
#include <iostream>

struct VarjoRenderTarget
{
    GLuint colorTextureId;
    GLuint depthTextureId;
    uint32_t width;
    uint32_t height;
};

class Varjo
{
public:
    varjo_Session *m_session = nullptr;
    int32_t m_viewCount;
    std::vector<varjo_Viewport> m_viewports;
    varjo_SwapChainConfig2 m_swapChainConfig;
    varjo_SwapChain *m_colorSwapChain = nullptr;
    std::vector<VarjoRenderTarget> m_renderTargets;
    int32_t m_currentSwapChainIndex = 0;
    std::vector<varjo_LayerMultiProjView> m_projectionLayers;
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
        std::vector<varjo_Viewport> viewports;
        viewports.reserve(m_viewCount);
        int x = 0, y = 0;
        for (int32_t i = 0; i < m_viewCount; i++)
        {
            const varjo_ViewDescription viewDescription = varjo_GetViewDescription(m_session, i);
            const varjo_Viewport viewport = varjo_Viewport{x, y, viewDescription.width, viewDescription.height};
            m_viewports.push_back(viewport);
            x += viewport.width;
            if (i > 0 && viewports.size() % 2 == 0)
            {
                x = 0;
                y += viewport.height;
            }
        }

        // Setup color swap chain (ring buffer of render targets).
        m_swapChainConfig.numberOfTextures = 3;
        m_swapChainConfig.textureArraySize = 1;
        m_swapChainConfig.textureFormat = varjo_TextureFormat_R8G8B8A8_SRGB;
        m_swapChainConfig.textureWidth = m_viewports.back().width + m_viewports.back().x;
        m_swapChainConfig.textureHeight = m_viewports.back().height + m_viewports.back().y;
        m_colorSwapChain = varjo_GLCreateSwapChain(m_session, &m_swapChainConfig);
        validate();

        // Create a render target per swap chain texture.
        for (int i = 0; i < m_swapChainConfig.numberOfTextures; ++i)
        {
            const varjo_Texture colorTexture = varjo_GetSwapChainImage(m_colorSwapChain, i);
            //XXX: create depth swapchain
            //const varjo_Texture depthTexture = varjo_GetSwapChainImage(m_depthSwapChain, i);
            m_renderTargets.push_back(
                VarjoRenderTarget{
                    varjo_ToGLTexture(colorTexture),
                    0, //varjo_ToGLTexture(depthTexture),
                    static_cast<uint32_t>(m_swapChainConfig.textureWidth),
                    static_cast<uint32_t>(m_swapChainConfig.textureHeight)});
        }

        // Create projection layers views
        m_projectionLayers.reserve(m_viewCount);
        for (int32_t i = 0; i < m_viewCount; i++)
        {
            m_projectionLayers[i].extension = nullptr;
            m_projectionLayers[i].viewport = varjo_SwapChainViewport{m_colorSwapChain, m_viewports[i].x, m_viewports[i].y, m_viewports[i].width, m_viewports[i].height, 0};
        }

        // Create a FrameInfo (used during main loop.)
        m_frameInfo = varjo_CreateFrameInfo(m_session);
        validate();
    }

    ~Varjo()
    {
        varjo_FreeFrameInfo(m_frameInfo);
        varjo_FreeSwapChain(m_colorSwapChain);
        varjo_SessionShutDown(m_session);
    }

    void beginFrameSync()
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
            for (uint32_t i = 0; i < m_viewCount; ++i)
            {
                varjo_ViewInfo &view = m_frameInfo->views[i];
                if (!view.enabled)
                {
                    continue; // Skip a view if it is not enabled.
                }

                //...

                std::copy(view.projectionMatrix, view.projectionMatrix + 16, m_projectionLayers[i].projection.value);
                std::copy(view.viewMatrix, view.viewMatrix + 16, m_projectionLayers[i].view.value);
            }
        }
    }

    void endFrame()
    {
        varjo_LayerMultiProj multiProjectionLayer{
            {varjo_LayerMultiProjType, 0}, varjo_SpaceLocal, static_cast<int32_t>(m_viewCount), m_projectionLayers.data()};
        std::array<varjo_LayerHeader *, 1> layers = {&multiProjectionLayer.header};
        varjo_SubmitInfoLayers submitInfoLayers{m_frameInfo->frameNumber, 0, 1, layers.data()};

        varjo_ReleaseSwapChainImage(m_colorSwapChain);

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

API_EXPORT const char *varjo_begin_frame_sync(Varjo *varjo)
{
    assert(varjo != nullptr && "Varjo instance expected");
    try
    {
        varjo->beginFrameSync();
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
