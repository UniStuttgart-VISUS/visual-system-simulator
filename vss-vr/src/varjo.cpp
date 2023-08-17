#include <Windows.h>

#define VULKAN_HPP_DISPATCH_LOADER_DYNAMIC 1
#define VULKAN_HPP_TYPESAFE_CONVERSION
#ifdef _WIN32
#define VK_USE_PLATFORM_WIN32_KHR
#endif
#include <vulkan/vulkan.hpp>

VULKAN_HPP_DEFAULT_DISPATCH_LOADER_DYNAMIC_STORAGE

#include <Varjo.h>
#include <Varjo_vk.h>
#include <Varjo_layers.h>
#include <Varjo_types_layers.h>
#include <cassert>
#include <stdexcept>
#include <vector>
#include <array>

#include <time.h>

// TODO instead of getting these values from wgpu we need to create and provide them here
struct VulkanData {
    VkInstance instance;
    VkDevice device;
    uint32_t queueFamilyIndex;
    uint32_t queueIndex;
};

struct VarjoRenderTarget {
    VkImage colorImage;
    VkImage depthImage;
    uint32_t width;
    uint32_t height;
};

struct VarjoGazeData {
    float leftEye[3];
    float rightEye[3];
    float focusDistance;
};

const float sample_gaze_data[] = {
0.13350107,0.05402988,0.9895748,0.073162355,0.0110148005,0.9972592,0.71261877,
-0.0030489685,-0.105599456,0.9944041,-0.10721052,-0.13200203,0.9854346,0.58676636,
0.46794492,0.07219766,0.88080364,0.42307574,0.030562863,0.90557873,0.6745882,
0.18453103,0.08754768,0.9789197,0.14879914,0.07552041,0.98597944,2.0,
0.32114023,0.02130485,0.946792,0.29801303,0.0043150443,0.95455205,2.0,
0.07173916,0.19369411,0.9784355,0.017741187,0.18021858,0.98346657,1.1338202,
0.3510931,-0.025078973,0.93600464,0.30433625,-0.062165186,0.95053405,0.81255543,
-0.1112047,0.16753642,0.9795739,-0.20003304,0.15159176,0.96799105,0.70306385,
-0.18110614,-0.1524473,0.9715762,-0.2442868,-0.18679225,0.9515422,0.7431514,
0.053587206,-0.032249358,0.9980423,-0.011419386,-0.043652996,0.9989815,0.9781806,
0.12828833,0.003226207,0.99173164,-0.006014107,-0.01734356,0.9998315,0.47441828,
0.058808092,-0.104745604,0.99275875,-0.006061984,-0.14311719,0.9896872,0.7472944,
0.3099795,-0.016338855,0.9506028,0.27556214,-0.039192792,0.96048397,2.0,
0.11435255,0.06772703,0.9911289,0.057687763,0.05842233,0.99662375,1.1164944,
0.03784335,0.03858852,0.9985383,-0.13847023,-0.080619186,0.9870798,0.25313002,
-0.13060987,-0.011663136,0.99136525,-0.21226238,-0.036445826,0.9765328,0.7133271,
0.36777252,0.044617433,0.9288448,0.335691,0.044672307,0.9409123,2.0,
0.15432747,0.07544118,0.9851354,0.08789092,0.060244214,0.9943067,0.920993,
0.22377588,-0.029064173,0.97420716,0.12696604,-0.053347066,0.9904715,0.6201661,
0.036011655,0.00323195,0.99934614,-0.024811324,-0.011234707,0.999629,1.0201786,
0.007960857,-0.03053517,0.999502,-0.06577903,-0.03595935,0.99718606,0.88327295,
0.25935382,0.07409501,0.96293586,0.19603242,0.050204687,0.9793114,0.8561375,
-0.013361443,-0.07222042,0.9972992,-0.07754328,-0.10840422,0.99107796,0.76949894,
0.23678564,0.02032961,0.9713492,0.1685139,0.018311907,0.9855292,0.91971487,
0.17588519,0.10851299,0.9784116,0.11417933,0.09789834,0.9886248,1.0051943,
-0.029578047,0.023962457,0.9992752,-0.09158822,-0.005134216,0.99578375,0.86416745,
0.20535997,0.2522541,0.9456189,0.18411095,0.2927691,0.9382907,2.0,
0.04779644,0.3268422,0.94386953,-0.020878065,0.33345357,0.94253534,0.94545525,
0.12674566,-0.19637074,0.9723035,0.034955703,-0.22613285,0.9734691,0.6457808};

class Varjo {
public:
    varjo_Session *m_session = nullptr;
    int32_t m_viewCount;
    std::vector<varjo_Viewport> m_viewports;
    varjo_SwapChainConfig2 m_colorSwapChainConfig;
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

    vk::Instance vkInstance;
    vk::PhysicalDevice vkPhysicalDevice;
    vk::Device vkDevice;
    vk::Queue graphicsQueue;
    uint32_t graphicsQueueFamily = -1;

    void validate() {
        varjo_Error error = varjo_GetError(m_session);
        if (error != varjo_NoError)
            throw std::runtime_error(varjo_GetErrorDesc(error));
    }

public:
    Varjo(VulkanData* vulkan_data) {
        // Test if Varjo system is available.
        if (!varjo_IsAvailable())
            throw std::runtime_error("Varjo system is unavailable");

        // Create session.
        m_session = varjo_SessionInit();
        validate();

        // printf("cpp-side: instance: %p, device: %p\n", vulkan_data->instance, vulkan_data->device);
        vk::DynamicLoader vkLoader;
        const PFN_vkGetInstanceProcAddr vkGetInstanceProcAddr = vkLoader.getProcAddress<PFN_vkGetInstanceProcAddr>("vkGetInstanceProcAddr");
        VULKAN_HPP_DEFAULT_DISPATCHER.init(vkGetInstanceProcAddr);
        // printf("VULKAN_HPP_DEFAULT_DISPATCHER.init done\n");
        
        createInstance(m_session);
        createDevice(m_session);

        // Enumerate and pack views into an atlas-like layout.
        m_viewCount = varjo_GetViewCount(m_session);
        m_viewports.reserve(m_viewCount);
        int x = 0, y = 0;
        int textureWidth = 0, textureHeight = 0;
        for (int32_t i = 0; i < m_viewCount; i++) {
            const varjo_ViewDescription viewDescription = varjo_GetViewDescription(m_session, i);
            const varjo_Viewport viewport = varjo_Viewport{x, y, viewDescription.width, viewDescription.height};
            // int32_t width_rec, height_rec;
            // varjo_GetTextureSize(m_session, varjo_TextureSize_Type_Recommended, i, &width_rec, &height_rec);
            // int32_t width_best, height_best;
            // varjo_GetTextureSize(m_session, varjo_TextureSize_Type_Recommended, i, &width_best, &height_best);
            // printf("cpp-side viewports: %i, %i recommended: %i, %i best: %i, %i\n", viewport.width, viewport.height, width_rec, height_rec, width_best, height_best);
            m_viewports.push_back(viewport);
            x += viewport.width;
            textureWidth = x > textureWidth ? x : textureWidth;
            // every two views we jump to the next "row"
            if (i % 2 == 1){
                x = 0;
                y += viewport.height;
                textureHeight = y > textureHeight ? y : textureHeight;
            }
        }

        // Setup color swap chain (ring buffer of render targets).
        m_colorSwapChainConfig = varjo_SwapChainConfig2 {
            varjo_TextureFormat_B8G8R8A8_SRGB,  // textureFormat
            4,                                  // numberOfTextures
            textureWidth,                       // textureWidth
            textureHeight,                      // textureHeight
            1                                   // textureArraySize
        };
        m_colorSwapChain = varjo_VKCreateSwapChain(m_session, vkDevice, graphicsQueueFamily, 0, &m_colorSwapChainConfig);

        m_depthSwapChainConfig = m_colorSwapChainConfig;
        m_depthSwapChainConfig.textureFormat = varjo_DepthTextureFormat_D24_UNORM_S8_UINT;
        m_depthSwapChain = varjo_VKCreateSwapChain(m_session, vkDevice, graphicsQueueFamily, 0, &m_depthSwapChainConfig);
        validate();

        // Create a render target per swap chain texture.
        for (int i = 0; i < m_colorSwapChainConfig.numberOfTextures; ++i) {
            const varjo_Texture colorTexture = varjo_GetSwapChainImage(m_colorSwapChain, i);
            const varjo_Texture depthTexture = varjo_GetSwapChainImage(m_depthSwapChain, i);
            m_renderTargets.push_back(
                VarjoRenderTarget{
                    varjo_ToVkTexture(colorTexture),
                    varjo_ToVkTexture(depthTexture),
                    static_cast<uint32_t>(textureWidth),
                    static_cast<uint32_t>(textureHeight)});
        }

        // Create projection layers views
        m_projectionLayers.reserve(m_viewCount);
        m_viewMatrices.reserve(m_viewCount*16);
        m_projMatrices.reserve(m_viewCount*16);
        for (int32_t i = 0; i < m_viewCount; i++) {
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
                printf("Using default Gaze Data; Failed to initialize Gaze: %s\n", varjo_GetErrorDesc(err));
            } else {
                m_gazeAvailable = true;
                m_gazeData.leftEye[0] = 0.0;
                m_gazeData.leftEye[1] = 0.0;
                m_gazeData.leftEye[2] = 1.0;
                m_gazeData.rightEye[0] = 0.0;
                m_gazeData.rightEye[1] = 0.0;
                m_gazeData.rightEye[2] = 1.0;
                m_gazeData.focusDistance = 1.0;
            }
        } else {
            printf("Using default Gaze Data; Gaze tracking is not allowed!\n");
        }
    }

    ~Varjo()
    {
        varjo_FreeFrameInfo(m_frameInfo);
        varjo_FreeSwapChain(m_colorSwapChain);
        varjo_FreeSwapChain(m_depthSwapChain);
        varjo_SessionShutDown(m_session);
    }

    
    void createInstance(varjo_Session* session) {
        int instanceExtensionCount = 0;
        varjo_GetInstanceExtensionsVk(m_session, &instanceExtensionCount, nullptr);
        std::vector<const char*> enabledInstanceExtensions(instanceExtensionCount);
        varjo_GetInstanceExtensionsVk(m_session, &instanceExtensionCount, enabledInstanceExtensions.data());
        printf("Required Vulkan instance extensions:\n");
        for (const auto& ext : enabledInstanceExtensions) {
            printf("    %s\n", ext);
        }

        enabledInstanceExtensions.push_back(VK_EXT_DEBUG_UTILS_EXTENSION_NAME);

        // TODO: automatically adjust these to wgpu requirements
        enabledInstanceExtensions.push_back(VK_KHR_SURFACE_EXTENSION_NAME);
        enabledInstanceExtensions.push_back(VK_KHR_WIN32_SURFACE_EXTENSION_NAME);
        //enabledInstanceExtensions.push_back(VK_KHR_SWAPCHAIN_MUTABLE_FORMAT_EXTENSION_NAME);
        enabledInstanceExtensions.push_back(VK_EXT_SWAPCHAIN_COLOR_SPACE_EXTENSION_NAME);

        printf("Enabled Vulkan instance extensions:\n");
        for (const auto& ext : enabledInstanceExtensions) {
            printf("    %s\n", ext);
        }

        const std::vector<const char*> enabledLayers = {
            "VK_LAYER_KHRONOS_validation"
        };

        const auto applicationInfo = vk::ApplicationInfo().setApiVersion(VK_MAKE_VERSION(1, 3, 0));
        const vk::InstanceCreateInfo info = vk::InstanceCreateInfo()
                                                .setPEnabledLayerNames(enabledLayers)
                                                .setPEnabledExtensionNames(enabledInstanceExtensions)
                                                .setPApplicationInfo(&applicationInfo);
        vkInstance = vk::createInstance(info);
        //if (res != vk::Result::eSuccess) {
        //    fprintf(stderr, "Failed to create a Vulkan instance, error code = %x", res);
        //    return false;
        //}

        VULKAN_HPP_DEFAULT_DISPATCHER.init(vkInstance);

        return;
    }

    void createDevice(varjo_Session* session) {
        vkPhysicalDevice = varjo_GetPhysicalDeviceVk(session, vkInstance);
        findQueueFamilies(vkPhysicalDevice);

        const float priority = 1.f;
        std::vector<vk::DeviceQueueCreateInfo> queueDesc;
        for (const int queueFamily : {graphicsQueueFamily}) {
            queueDesc.push_back(vk::DeviceQueueCreateInfo().setQueueFamilyIndex(queueFamily).setQueueCount(1).setPQueuePriorities(&priority));
        }

        // TODO: automatically adjust these to wgpu requirements
        const auto deviceFeatures = vk::PhysicalDeviceFeatures().setRobustBufferAccess(VK_TRUE).setImageCubeArray(VK_TRUE).setIndependentBlend(VK_TRUE).setSampleRateShading(VK_TRUE).setSamplerAnisotropy(VK_TRUE).setFragmentStoresAndAtomics(VK_TRUE);

        // TODO: automatically adjust these to wgpu requirements
        auto framebufferFeatures = vk::PhysicalDeviceImagelessFramebufferFeatures().setImagelessFramebuffer(VK_TRUE);
        auto semaphoreFeatures = vk::PhysicalDeviceTimelineSemaphoreFeatures().setTimelineSemaphore(VK_TRUE).setPNext(&framebufferFeatures);
        auto robustnessFeatures = vk::PhysicalDeviceImageRobustnessFeatures().setRobustImageAccess(VK_TRUE).setPNext(&semaphoreFeatures);
        auto synchronization2Features = vk::PhysicalDeviceSynchronization2Features().setSynchronization2(VK_TRUE).setPNext(&robustnessFeatures);
        auto robustness2Features = vk::PhysicalDeviceRobustness2FeaturesEXT().setRobustBufferAccess2(VK_TRUE).setRobustImageAccess2(VK_TRUE).setPNext(&synchronization2Features);
        auto memoryFeatures = vk::PhysicalDeviceZeroInitializeWorkgroupMemoryFeatures().setShaderZeroInitializeWorkgroupMemory(VK_TRUE).setPNext(&robustness2Features);

        int32_t deviceExtensionsCount = 0;
        varjo_GetDeviceExtensionsVk(session, &deviceExtensionsCount, nullptr);
        std::vector<const char*> extVec(deviceExtensionsCount);
        varjo_GetDeviceExtensionsVk(session, &deviceExtensionsCount, extVec.data());

        extVec.push_back(VK_KHR_MAINTENANCE_1_EXTENSION_NAME);
        extVec.push_back(VK_KHR_SYNCHRONIZATION_2_EXTENSION_NAME);

        // needed for wgpu
        extVec.push_back(VK_KHR_SWAPCHAIN_EXTENSION_NAME);
        //extVec.push_back(VK_KHR_IMAGELESS_FRAMEBUFFER_EXTENSION_NAME);

        printf("Enabled Vulkan device extensions:\n");
        for (const auto& ext : extVec) {
            printf("    %s\n", ext);
        }

        std::vector<char*> layerVec;

        const auto deviceDesc = vk::DeviceCreateInfo()
                                    .setQueueCreateInfos(queueDesc)
                                    .setPEnabledFeatures(&deviceFeatures)
                                    .setPEnabledExtensionNames(extVec)
                                    .setPEnabledLayerNames(layerVec)
                                    .setPNext(&memoryFeatures);

        vkDevice = vkPhysicalDevice.createDevice(deviceDesc);
        //if (res != vk::Result::eSuccess) {
        //    return false;
        //}

        VULKAN_HPP_DEFAULT_DISPATCHER.init(vkDevice);
        graphicsQueue = vkDevice.getQueue(graphicsQueueFamily, 0);
        // TODO Check for errors
        return;
    }

    void findQueueFamilies(vk::PhysicalDevice physicalDevice) {
        const auto props = physicalDevice.getQueueFamilyProperties();

        for (int i = 0; i < static_cast<int>(props.size()); i++) {
            const auto& queueFamily = props[i];

            if (graphicsQueueFamily == -1) {
                if (queueFamily.queueCount > 0 && (queueFamily.queueFlags & vk::QueueFlagBits::eGraphics)) {
                    graphicsQueueFamily = i;
                }
            }
        }

        return;
    }

    bool beginFrameSync() {
        varjo_Event evt;
        while (varjo_PollEvent(m_session, &evt)) {
            switch (evt.header.type) {
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
                if (evt.data.button.buttonId == varjo_ButtonId_Application && evt.data.button.pressed) {
                    //gaze.requestCalibration();
                }
                break;
            // case varjo_EventType_FoveationStatus:
            //     //renderer->useFoveatedViewports(evt.data.foveationStatus.status == varjo_FoveationStatus_Ok);
            //     break;
            }
        }
        if (m_visible) {
            // Wait before rendering the next frame.
            varjo_WaitSync(m_session, m_frameInfo);

            varjo_BeginFrameWithLayers(m_session);

            varjo_AcquireSwapChainImage(m_colorSwapChain, &m_currentSwapChainIndex);
            varjo_AcquireSwapChainImage(m_depthSwapChain, &m_currentSwapChainIndex);
            for (uint32_t i = 0; i < m_viewCount; ++i) {
                varjo_ViewInfo &view = m_frameInfo->views[i];
                if (!view.enabled) {
                    continue; // Skip a view if it is not enabled.
                }

                //...
                for(int j = 0; j < 16; ++j) {
                    m_viewMatrices[i*16+j] = view.viewMatrix[j];
                    m_projMatrices[i*16+j] = view.projectionMatrix[j];
                }

                std::copy(view.projectionMatrix, view.projectionMatrix + 16, m_projectionLayers[i].projection.value);
                std::copy(view.viewMatrix, view.viewMatrix + 16, m_projectionLayers[i].view.value);
            }
        }
        return m_visible;
    }

    void endFrame() {
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

API_EXPORT const char *varjo_new(Varjo **varjo, VulkanData *vulkan_data)
{
    assert(*varjo == nullptr && "Null pointer expected");
    try
    {
        *varjo = new Varjo(vulkan_data);
        *vulkan_data = VulkanData{
            (*varjo)->vkInstance,
            (*varjo)->vkDevice,
            (*varjo)->graphicsQueueFamily,
            0
        };
        printf("cpp-side: instance: %p, device: %p\n", (void *)vulkan_data->instance, (void *)vulkan_data->device);
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

API_EXPORT const char *varjo_viewports(Varjo *varjo, varjo_Viewport **viewports, int32_t *view_count, int32_t *texture_width, int32_t *texture_height)
{
    assert(varjo != nullptr && "Varjo instance expected");
    try
    {
        *viewports = varjo->m_viewports.data();
        *view_count = varjo->m_viewCount;
        *texture_width = varjo->m_colorSwapChainConfig.textureWidth;
        *texture_height = varjo->m_colorSwapChainConfig.textureHeight;
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

        if (varjo->m_gazeAvailable){
            varjo_SyncProperties(varjo->m_session);

            // Get gaze and check that it is valid
            varjo_Gaze gaze = varjo_GetGaze(varjo->m_session);
            if (gaze.status == varjo_GazeStatus_Invalid) return nullptr;

            if(gaze.leftStatus == varjo_GazeEyeStatus_Compensated || gaze.leftStatus == varjo_GazeEyeStatus_Tracked){
                varjo->m_gazeData.leftEye[0] = gaze.leftEye.forward[0];
                varjo->m_gazeData.leftEye[1] = gaze.leftEye.forward[1];
                varjo->m_gazeData.leftEye[2] = gaze.leftEye.forward[2];
            }
            if(gaze.rightStatus == varjo_GazeEyeStatus_Compensated || gaze.rightStatus == varjo_GazeEyeStatus_Tracked){
                varjo->m_gazeData.rightEye[0] = gaze.rightEye.forward[0];
                varjo->m_gazeData.rightEye[1] = gaze.rightEye.forward[1];
                varjo->m_gazeData.rightEye[2] = gaze.rightEye.forward[2];
            }
            varjo->m_gazeData.focusDistance = gaze.focusDistance;
        }else{
            time_t seconds = time(NULL);
            size_t index = (seconds % 29);
            varjo->m_gazeData.leftEye[0] = sample_gaze_data[index*7 + 0];
            varjo->m_gazeData.leftEye[1] = sample_gaze_data[index*7 + 1];
            varjo->m_gazeData.leftEye[2] = sample_gaze_data[index*7 + 2];
            varjo->m_gazeData.rightEye[0] = sample_gaze_data[index*7 + 3];
            varjo->m_gazeData.rightEye[1] = sample_gaze_data[index*7 + 4];
            varjo->m_gazeData.rightEye[2] = sample_gaze_data[index*7 + 5];
            varjo->m_gazeData.focusDistance = sample_gaze_data[index*7 + 6];
        }

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
