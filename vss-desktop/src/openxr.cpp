#include <openxr.h>
#include <openxr_platform.h>
#include <openxr_platform_defines.h>
#include <openxr_reflection.h>

#include <iostream>
#include <cassert>
#include <stdexcept>
#include <vector>

class OpenXR
{
public:

public:
    OpenXR(){}

    ~OpenXR(){}

    void initialize(){
        std::vector<const char*> extensions;

        // Transform platform and graphics extension std::strings to C strings.

        /*const std::vector<std::string> graphicsExtensions = {XR_KHR_OPENGL_ES_ENABLE_EXTENSION_NAME};
        std::transform(graphicsExtensions.begin(), graphicsExtensions.end(), std::back_inserter(extensions),
                       [](const std::string& ext) { return ext.c_str(); });

        XrInstanceCreateInfo createInfo{XR_TYPE_INSTANCE_CREATE_INFO};
        createInfo.next = nullptr;
        createInfo.enabledExtensionCount = (uint32_t)extensions.size();
        createInfo.enabledExtensionNames = extensions.data();

        strcpy(createInfo.applicationInfo.applicationName, "HelloXR");
        createInfo.applicationInfo.apiVersion = XR_CURRENT_API_VERSION;

        XrInstance_t m_instance;
        xrCreateInstance(&createInfo, &m_instance);*/

        //uint32_t ext_count = 0;
        //xrEnumerateInstanceExtensionProperties(nullptr, 0, &ext_count, nullptr);
        //printf("extension count: %i\n", ext_count);
    }
};

#define API_EXPORT extern "C"

API_EXPORT const char *openxr_new(OpenXR **openxr)
{
    assert(*openxr == nullptr && "Null pointer expected");
    try
    {
        *openxr = new OpenXR();
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

API_EXPORT const char *openxr_init(OpenXR *openxr)
{
    assert(openxr != nullptr && "OpenXR instance expected");
    try
    {
        openxr->initialize();
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