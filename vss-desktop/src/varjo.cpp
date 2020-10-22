#include <Windows.h>
#include <GL/GL.h>
#include <Varjo.h>
#include <Varjo_gl.h>
#include <Varjo_layers.h>
#include <Varjo_types_layers.h>
#include <cassert>
#include <stdexcept>
#include <vector>

class Varjo
{
private:
    varjo_Session *m_session = nullptr;
    std::vector<varjo_Viewport> m_viewports;
    varjo_SwapChainConfig2 m_swapChainConfig;
    varjo_SwapChain *m_colorSwapChain = nullptr;
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

        // Enumerate views.
        const auto viewCount = varjo_GetViewCount(m_session);
        std::vector<varjo_Viewport> viewports;
        viewports.reserve(viewCount);
        int x = 0, y = 0;
        for (int32_t i = 0; i < viewCount; i++)
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

        // Setup color swap chain.
        m_swapChainConfig.numberOfTextures = 4;
        m_swapChainConfig.textureArraySize = 1;
        m_swapChainConfig.textureFormat = varjo_TextureFormat_R8G8B8A8_SRGB;
        m_swapChainConfig.textureWidth = m_viewports.back().width + m_viewports.back().x;
        m_swapChainConfig.textureHeight = m_viewports.back().height + m_viewports.back().y;
        m_colorSwapChain = varjo_GLCreateSwapChain(m_session, &m_swapChainConfig);
        validate();

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

            /*
            varjo_BeginFrameWithLayers(m_session);

            int32_t swapChainIndex = 0;
            varjo_AcquireSwapChainImage(m_colorSwapChain, &swapChainIndex);
            m_currentRenderTarget = m_renderTargets[swapChainIndex];
            for (uint32_t i = 0; i < m_viewCount; ++i)
            {
                varjo_ViewInfo &view = frameInfo->views[i];
                if (!view.enabled)
                {
                    continue; // Skip a view if it is not enabled.
                }

                //...
            }
            */
        }
    }

    void endFrame()
    {
        /*
        varjo_ReleaseSwapChainImage(m_colorSwapChain);

        varjo_LayerMultiProj multiProjectionLayer{
            {varjo_LayerMultiProjType, flags}, varjo_SpaceLocal, static_cast<int32_t>(m_viewCount), m_multiprojectionViews.data()};
        std::array<varjo_LayerHeader *, 1> layers = {&multiProjectionLayer.header};
        varjo_SubmitInfoLayers submitInfoLayers{frameInfo->frameNumber, 0, m_colorSwapChain != nullptr ? 1 : 0, layers.data()};

        varjo_EndFrameWithLayers(m_session, &submitInfoLayers);
        */
    }

    const varjo_FrameInfo &frameInfo() const
    {
        return *m_frameInfo;
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
}

API_EXPORT void varjo_drop(Varjo **varjo)
{
    delete *varjo;
    *varjo = nullptr;
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
}
