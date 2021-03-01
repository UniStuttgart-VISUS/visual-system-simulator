#define XR_USE_GRAPHICS_API_OPENGL
#define XR_USE_PLATFORM_WIN32

#include "windows.h"
#include <GL/gl.h>

#include <openxr.h>
#include <openxr_platform.h>
#include <openxr_platform_defines.h>
#include <openxr_reflection.h>

#include <iostream>
#include <cassert>
#include <stdexcept>
#include <vector>
#include <cstdlib>
#include <algorithm>


class OpenXR
{
private:
    XrInstance     xr_instance      = {};
    // Function pointers for some OpenXR extension methods we'll use.
    PFN_xrGetOpenGLGraphicsRequirementsKHR  ext_xrGetD3D11GraphicsRequirementsKHR   = nullptr;
    PFN_xrCreateDebugUtilsMessengerEXT      ext_xrCreateDebugUtilsMessengerEXT      = nullptr;
    PFN_xrDestroyDebugUtilsMessengerEXT     ext_xrDestroyDebugUtilsMessengerEXT     = nullptr;
    XrSystemId                              xr_system_id                            = XR_NULL_SYSTEM_ID;
    XrFormFactor                            app_config_form                         = XR_FORM_FACTOR_HEAD_MOUNTED_DISPLAY;
    XrDebugUtilsMessengerEXT                xr_debug                                = {};
    XrEnvironmentBlendMode                  xr_blend                                = {};
    std::vector<XrViewConfigurationView>    xr_config_views;
    XrViewConfigurationType                 app_config_view                         = XR_VIEW_CONFIGURATION_TYPE_PRIMARY_STEREO;
    XrSession      xr_session       = {};
public:

public:
    OpenXR(){}

    ~OpenXR(){}

    void initialize(){
        std::vector<const char*> extensions;        

        const char  *ask_extensions[] = { 
            XR_KHR_OPENGL_ENABLE_EXTENSION_NAME, 
            XR_EXT_DEBUG_UTILS_EXTENSION_NAME,
        };

        printf("We need %s\n", XR_KHR_OPENGL_ENABLE_EXTENSION_NAME);


        uint32_t ext_count = 0;
        xrEnumerateInstanceExtensionProperties(nullptr, 0, &ext_count, nullptr);
        std::vector<XrExtensionProperties> xr_exts(ext_count, { XR_TYPE_EXTENSION_PROPERTIES });
        xrEnumerateInstanceExtensionProperties(nullptr, ext_count, &ext_count, xr_exts.data());

        printf("OpenXR extensions available:\n");
        for (size_t i = 0; i < xr_exts.size(); i++) {
            printf("- %s\n", xr_exts[i].extensionName);

            // Check if we're asking for this extensions, and add it to our use 
            // list!
            for (int32_t ask = 0; ask < _countof(ask_extensions); ask++) {
                if (strcmp(ask_extensions[ask], xr_exts[i].extensionName) == 0) {
                    extensions.push_back(ask_extensions[ask]);
                    break;
                }
            }
        }

        // If a required extension isn't present, you want to ditch out here!
        // It's possible something like your rendering API might not be provided
        // by the active runtime. APIs like OpenGL don't have universal support.
        if (!std::any_of( extensions.begin(), extensions.end(), 
            [] (const char *ext) {
                return strcmp(ext, XR_KHR_OPENGL_ENABLE_EXTENSION_NAME)==0;
            }))
            exit(1);


        XrInstanceCreateInfo createInfo = { XR_TYPE_INSTANCE_CREATE_INFO };
        createInfo.enabledExtensionCount      = extensions.size();
        createInfo.enabledExtensionNames      = extensions.data();
        createInfo.applicationInfo.apiVersion = XR_CURRENT_API_VERSION;
        strcpy_s(createInfo.applicationInfo.applicationName, "vss_was_sonst");
        xrCreateInstance(&createInfo, &xr_instance);


        // Check if OpenXR is on this system, if this is null here, the user 
        // needs to install an OpenXR runtime and ensure it's active!
        if (xr_instance == nullptr)
            exit(1);


        // Load extension methods that we'll need for this application! There's a
        // couple ways to do this, and this is a fairly manual one. Chek out this
        // file for another way to do it:
        // https://github.com/maluoi/StereoKit/blob/master/StereoKitC/systems/platform/openxr_extensions.h
        xrGetInstanceProcAddr(xr_instance, "xrCreateDebugUtilsMessengerEXT",    (PFN_xrVoidFunction *)(&ext_xrCreateDebugUtilsMessengerEXT   ));
        xrGetInstanceProcAddr(xr_instance, "xrDestroyDebugUtilsMessengerEXT",   (PFN_xrVoidFunction *)(&ext_xrDestroyDebugUtilsMessengerEXT  ));
        xrGetInstanceProcAddr(xr_instance, "xrGetD3D11GraphicsRequirementsKHR", (PFN_xrVoidFunction *)(&ext_xrGetD3D11GraphicsRequirementsKHR));

        // Set up a really verbose debug log! Great for dev, but turn this off or
        // down for final builds. WMR doesn't produce much output here, but it
        // may be more useful for other runtimes?
        // Here's some extra information about the message types and severities:
        // https://www.khronos.org/registry/OpenXR/specs/1.0/html/xrspec.html#debug-message-categorization
        XrDebugUtilsMessengerCreateInfoEXT debug_info = { XR_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT };
        debug_info.messageTypes =
            XR_DEBUG_UTILS_MESSAGE_TYPE_GENERAL_BIT_EXT     |
            XR_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT  |
            XR_DEBUG_UTILS_MESSAGE_TYPE_PERFORMANCE_BIT_EXT |
            XR_DEBUG_UTILS_MESSAGE_TYPE_CONFORMANCE_BIT_EXT;
        debug_info.messageSeverities =
            XR_DEBUG_UTILS_MESSAGE_SEVERITY_VERBOSE_BIT_EXT |
            XR_DEBUG_UTILS_MESSAGE_SEVERITY_INFO_BIT_EXT    |
            XR_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT |
            XR_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT;
        debug_info.userCallback = [](XrDebugUtilsMessageSeverityFlagsEXT severity, XrDebugUtilsMessageTypeFlagsEXT types, const XrDebugUtilsMessengerCallbackDataEXT *msg, void* user_data) {
            // Print the debug message we got! There's a bunch more info we could
            // add here too, but this is a pretty good start, and you can always
            // add a breakpoint this line!
            printf("%s: %s\n", msg->functionName, msg->message);

            // Output to debug window
            char text[512];
            sprintf_s(text, "%s: %s", msg->functionName, msg->message);
            OutputDebugStringA(text);

            // Returning XR_TRUE here will force the calling function to fail
            return (XrBool32)XR_FALSE;
        };
        // Start up the debug utils!
        if (ext_xrCreateDebugUtilsMessengerEXT)
            ext_xrCreateDebugUtilsMessengerEXT(xr_instance, &debug_info, &xr_debug);
        
        // Request a form factor from the device (HMD, Handheld, etc.)
        XrSystemGetInfo systemInfo = { XR_TYPE_SYSTEM_GET_INFO };
        systemInfo.formFactor = app_config_form;
        xrGetSystem(xr_instance, &systemInfo, &xr_system_id);

        // Check what blend mode is valid for this device (opaque vs transparent displays)
        // We'll just take the first one available!
        uint32_t blend_count = 0;
        xrEnumerateEnvironmentBlendModes(xr_instance, xr_system_id, app_config_view, 1, &blend_count, &xr_blend);

        // // OpenXR wants to ensure apps are using the correct graphics card, so this MUST be called 
        // // before xrCreateSession. This is crucial on devices that have multiple graphics cards, 
        // // like laptops with integrated graphics chips in addition to dedicated graphics cards.
       
       // this segfaults
        XrGraphicsRequirementsOpenGLKHR requirement = { XR_TYPE_GRAPHICS_REQUIREMENTS_D3D11_KHR };
        ext_xrGetD3D11GraphicsRequirementsKHR(xr_instance, xr_system_id, &requirement);

        // ignore the requirements for now








































        // Transform platform and graphics extension std::strings to C strings.

        // const std::vector<std::string> graphicsExtensions = {XR_KHR_OPENGL_ENABLE_EXTENSION_NAME};

        // std::transform(graphicsExtensions.begin(), graphicsExtensions.end(), std::back_inserter(extensions),
        //                [](const std::string& ext) { return ext.c_str(); });

        /*
        XrInstanceCreateInfo createInfo{XR_TYPE_INSTANCE_CREATE_INFO};
        createInfo.next = nullptr;
        createInfo.enabledExtensionCount = (uint32_t)extensions.size();
        createInfo.enabledExtensionNames = extensions.data();

        strcpy(createInfo.applicationInfo.applicationName, "HelloXR");
        createInfo.applicationInfo.apiVersion = XR_CURRENT_API_VERSION;

        XrInstance_t m_instance;
        xrCreateInstance(&createInfo, &m_instance);
        */
        
        // uint32_t ext_count = 0;
        // xrEnumerateInstanceExtensionProperties(nullptr, 0, &ext_count, nullptr);
        // printf("extension count: %i\n", ext_count);
    }

    void create_session(){
        // A session represents this application's desire to display things! This is where we hook up our graphics API.
        // This does not start the session, for that, you'll need a call to xrBeginSession, which we do in openxr_poll_events
        XrGraphicsBindingOpenGLWin32KHR binding = { XR_TYPE_GRAPHICS_BINDING_OPENGL_WIN32_KHR };
        // binding.gDC = XXXXXX;       // hDC is a valid Windows HW device context handle.
        // binding.hGLRC = XXXXXX;     // hGLRC is a valid Windows OpenGL rendering context handle.

        XrSessionCreateInfo sessionInfo = { XR_TYPE_SESSION_CREATE_INFO };
        sessionInfo.next     = &binding;
        sessionInfo.systemId = xr_system_id;
        xrCreateSession(xr_instance, &sessionInfo, &xr_session);

        // Unable to start a session, may not have an MR device attached or ready
        if (xr_session == nullptr)
            exit(1);
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

API_EXPORT const char *openxr_create_session(OpenXR *openxr)
{
    assert(openxr != nullptr && "OpenXR instance expected");
    try
    {
        openxr->create_session();
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